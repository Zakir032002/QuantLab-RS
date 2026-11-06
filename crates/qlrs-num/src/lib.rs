//! Checked fixed-point decimal arithmetic. No dependencies and no floating point.
//!
//! Scale belongs to each value. Multiplication uses a checked signed integer
//! fast path and a cold unsigned 256-bit path when the intermediate is wide.
//!
//! ```
//! use qlrs_num::{Fixed, Rounding};
//! let cash: Fixed = "10000.00".parse()?;
//! let quantity: Fixed = "20".parse()?;
//! let price: Fixed = "100.00".parse()?;
//! let fee: Fixed = "10.00".parse()?;
//! let notional = quantity.mul_rescale(price, 2, Rounding::Exact)?;
//! let remaining = cash.checked_sub(notional)?.checked_sub(fee)?;
//! assert_eq!(remaining.to_string(), "7990.00");
//! # Ok::<(), qlrs_num::NumericError>(())
//! ```
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic, clippy::arithmetic_side_effects)]

#[cfg(kani)]
mod proofs;
use std::{cmp::Ordering, error::Error, fmt, str::FromStr};

/// Maximum supported scale of a stored value.
pub const MAX_SCALE: u8 = 18;

/// A decimal integer with an explicit per-value scale.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Fixed {
    raw: i128,
    scale: u8,
}

/// An explicitly requested rounding policy. Down means toward zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rounding {
    /// Refuse to discard any nonzero digits.
    Exact,
    /// Round to nearest, with ties to an even last digit.
    HalfEven,
    /// Round to nearest, with ties away from zero.
    HalfUp,
    /// Discard fractional digits, toward zero.
    Down,
}

/// A numeric failure; callers must propagate it as evidence, not a panic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericError {
    /// Invalid decimal-string syntax.
    InvalidDecimal,
    /// The per-value scale exceeds the Part 1 range.
    UnsupportedScale,
    /// The final raw value does not fit in a signed 128-bit integer.
    Overflow,
    /// Exact rescaling would discard nonzero digits.
    RoundingRequired,
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDecimal => "invalid-decimal",
            Self::UnsupportedScale => "unsupported-scale",
            Self::Overflow => "numeric-overflow",
            Self::RoundingRequired => "rounding-policy-required",
        })
    }
}

impl Error for NumericError {}

impl Fixed {
    /// Construct a value, checking the stored scale.
    ///
    /// # Errors
    /// Returns `UnsupportedScale` for scales greater than 18.
    pub const fn new(raw: i128, scale: u8) -> Result<Self, NumericError> {
        if scale > MAX_SCALE {
            Err(NumericError::UnsupportedScale)
        } else {
            Ok(Self { raw, scale })
        }
    }

    /// The signed raw integer.
    #[must_use]
    pub const fn raw(self) -> i128 {
        self.raw
    }

    /// The number of fractional digits, including trailing zeroes.
    #[must_use]
    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// Parse a decimal with a declared maximum scale, without allocating.
    ///
    /// # Errors
    /// Rejects invalid syntax, excess scale, and integer overflow.
    pub fn parse(input: &str, max_scale: u8) -> Result<Self, NumericError> {
        let (negative, digits) = input
            .strip_prefix('-')
            .map_or((false, input), |digits| (true, digits));
        let mut raw = 0_i128;
        let mut scale = 0_u8;
        let mut point = false;
        let mut integer_digit = false;
        let mut fractional_digit = false;
        for byte in digits.bytes() {
            if byte == b'.' && !point && integer_digit {
                point = true;
                continue;
            }
            if !byte.is_ascii_digit() {
                return Err(NumericError::InvalidDecimal);
            }
            let digit = i128::from(byte.wrapping_sub(b'0'));
            raw = raw.checked_mul(10).ok_or(NumericError::Overflow)?;
            raw = if negative {
                raw.checked_sub(digit)
            } else {
                raw.checked_add(digit)
            }
            .ok_or(NumericError::Overflow)?;
            if point {
                fractional_digit = true;
                scale = scale.checked_add(1).ok_or(NumericError::UnsupportedScale)?;
                if scale > max_scale || scale > MAX_SCALE {
                    return Err(NumericError::UnsupportedScale);
                }
            } else {
                integer_digit = true;
            }
        }
        if !integer_digit || (point && !fractional_digit) {
            return Err(NumericError::InvalidDecimal);
        }
        Self::new(raw, scale)
    }

    /// Checked addition at the higher operand scale.
    ///
    /// # Errors
    /// Returns `Overflow` when either alignment or the sum overflows.
    pub fn checked_add(self, rhs: Self) -> Result<Self, NumericError> {
        let scale = self.scale.max(rhs.scale);
        let left = self.rescale(scale, Rounding::Exact)?;
        let right = rhs.rescale(scale, Rounding::Exact)?;
        Self::new(
            left.raw
                .checked_add(right.raw)
                .ok_or(NumericError::Overflow)?,
            scale,
        )
    }

    /// Checked subtraction at the higher operand scale.
    ///
    /// # Errors
    /// Returns `Overflow` when either alignment or the difference overflows.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, NumericError> {
        let scale = self.scale.max(rhs.scale);
        let left = self.rescale(scale, Rounding::Exact)?;
        let right = rhs.rescale(scale, Rounding::Exact)?;
        Self::new(
            left.raw
                .checked_sub(right.raw)
                .ok_or(NumericError::Overflow)?,
            scale,
        )
    }

    /// Multiply exactly at the sum of operand scales.
    ///
    /// # Errors
    /// Returns `UnsupportedScale` if the product scale exceeds 18, or overflow.
    /// Use `mul_rescale` when a wider intermediate scale must be reduced.
    pub fn checked_mul(self, rhs: Self) -> Result<Self, NumericError> {
        let scale = self
            .scale
            .checked_add(rhs.scale)
            .ok_or(NumericError::UnsupportedScale)?;
        self.mul_rescale(rhs, scale, Rounding::Exact)
    }

    /// Compute `self * rhs + addend` exactly before a single final rounding.
    /// This avoids double rounding of notional and commission.
    ///
    /// # Errors
    /// Returns unsupported scale, final overflow, or required rounding errors.
    pub fn mul_add_rescale(
        self,
        rhs: Self,
        addend: Self,
        scale: u8,
        mode: Rounding,
    ) -> Result<Self, NumericError> {
        if scale > MAX_SCALE {
            return Err(NumericError::UnsupportedScale);
        }
        let product_scale = self
            .scale
            .checked_add(rhs.scale)
            .ok_or(NumericError::UnsupportedScale)?;
        let common = product_scale.max(addend.scale);
        let product_factor = 10_i128
            .checked_pow(u32::from(common.wrapping_sub(product_scale)))
            .ok_or(NumericError::Overflow)?;
        let add_factor = 10_i128
            .checked_pow(u32::from(common.wrapping_sub(addend.scale)))
            .ok_or(NumericError::Overflow)?;
        if let Some(sum) = self
            .raw
            .checked_mul(rhs.raw)
            .and_then(|p| p.checked_mul(product_factor))
            .zip(addend.raw.checked_mul(add_factor))
            .and_then(|(p, a)| p.checked_add(a))
        {
            return rescale_raw(sum, common, scale, mode);
        }
        wide_mul_add(self, rhs, addend, scale, mode)
    }

    /// Checked multiplication followed by an explicit rescale.
    ///
    /// # Errors
    /// Returns scale, final overflow, or inexact rounding errors. Intermediate
    /// multiplication overflow alone is handled by the cold wide path.
    pub fn mul_rescale(
        self,
        rhs: Self,
        scale: u8,
        rounding: Rounding,
    ) -> Result<Self, NumericError> {
        if scale > MAX_SCALE {
            return Err(NumericError::UnsupportedScale);
        }
        let product_scale = self
            .scale
            .checked_add(rhs.scale)
            .ok_or(NumericError::Overflow)?;
        if let Some(raw) = self.raw.checked_mul(rhs.raw) {
            return rescale_raw(raw, product_scale, scale, rounding);
        }
        wide_mul_rescale(self, rhs, scale, rounding)
    }

    /// Change the stored scale under an explicit rounding policy.
    ///
    /// # Errors
    /// Returns scale, overflow, or inexact rounding errors.
    pub fn rescale(self, scale: u8, rounding: Rounding) -> Result<Self, NumericError> {
        if scale > MAX_SCALE {
            return Err(NumericError::UnsupportedScale);
        }
        rescale_raw(self.raw, self.scale, scale, rounding)
    }

    /// Compare numeric values exactly, without downscaling either operand.
    #[must_use]
    pub fn numeric_cmp(self, rhs: Self) -> Ordering {
        if self.scale == rhs.scale {
            return self.raw.cmp(&rhs.raw);
        }
        let common_scale = self.scale.max(rhs.scale);
        if let (Ok(left), Ok(right)) = (
            self.rescale(common_scale, Rounding::Exact),
            rhs.rescale(common_scale, Rounding::Exact),
        ) {
            return left.raw.cmp(&right.raw);
        }
        let sign = self.raw.signum().cmp(&rhs.raw.signum());
        if sign != Ordering::Equal {
            return sign;
        }
        let scale = self.scale.max(rhs.scale);
        let left = wide_product(
            self.raw.unsigned_abs(),
            10_u128.pow(u32::from(scale.wrapping_sub(self.scale))),
        );
        let right = wide_product(
            rhs.raw.unsigned_abs(),
            10_u128.pow(u32::from(scale.wrapping_sub(rhs.scale))),
        );
        let result = left.cmp(&right);
        if self.raw.is_negative() {
            result.reverse()
        } else {
            result
        }
    }
}

impl FromStr for Fixed {
    type Err = NumericError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input, MAX_SCALE)
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.raw < 0 {
            f.write_str("-")?;
        }
        let magnitude = self.raw.unsigned_abs();
        if self.scale == 0 {
            return write!(f, "{magnitude}");
        }
        let factor = 10_u128.pow(u32::from(self.scale));
        let whole = magnitude.checked_div(factor).ok_or(fmt::Error)?;
        let fraction = magnitude.checked_rem(factor).ok_or(fmt::Error)?;
        write!(
            f,
            "{whole}.{fraction:0width$}",
            width = usize::from(self.scale)
        )
    }
}

fn rescale_raw(raw: i128, from: u8, to: u8, rounding: Rounding) -> Result<Fixed, NumericError> {
    if to >= from {
        let exponent = u32::from(to.wrapping_sub(from));
        let factor = 10_i128
            .checked_pow(exponent)
            .ok_or(NumericError::Overflow)?;
        return Fixed::new(raw.checked_mul(factor).ok_or(NumericError::Overflow)?, to);
    }
    divide_power(
        raw.unsigned_abs(),
        from.wrapping_sub(to),
        raw < 0,
        to,
        rounding,
    )
}

fn divide_power(
    magnitude: u128,
    power: u8,
    negative: bool,
    scale: u8,
    mode: Rounding,
) -> Result<Fixed, NumericError> {
    match power {
        1 => divide_const::<1>(magnitude, negative, scale, mode),
        2 => divide_const::<2>(magnitude, negative, scale, mode),
        3 => divide_const::<3>(magnitude, negative, scale, mode),
        4 => divide_const::<4>(magnitude, negative, scale, mode),
        5 => divide_const::<5>(magnitude, negative, scale, mode),
        6 => divide_const::<6>(magnitude, negative, scale, mode),
        7 => divide_const::<7>(magnitude, negative, scale, mode),
        8 => divide_const::<8>(magnitude, negative, scale, mode),
        9 => divide_const::<9>(magnitude, negative, scale, mode),
        10 => divide_const::<10>(magnitude, negative, scale, mode),
        11 => divide_const::<11>(magnitude, negative, scale, mode),
        12 => divide_const::<12>(magnitude, negative, scale, mode),
        13 => divide_const::<13>(magnitude, negative, scale, mode),
        14 => divide_const::<14>(magnitude, negative, scale, mode),
        15 => divide_const::<15>(magnitude, negative, scale, mode),
        16 => divide_const::<16>(magnitude, negative, scale, mode),
        17 => divide_const::<17>(magnitude, negative, scale, mode),
        18 => divide_const::<18>(magnitude, negative, scale, mode),
        19 => divide_const::<19>(magnitude, negative, scale, mode),
        20 => divide_const::<20>(magnitude, negative, scale, mode),
        21 => divide_const::<21>(magnitude, negative, scale, mode),
        22 => divide_const::<22>(magnitude, negative, scale, mode),
        23 => divide_const::<23>(magnitude, negative, scale, mode),
        24 => divide_const::<24>(magnitude, negative, scale, mode),
        25 => divide_const::<25>(magnitude, negative, scale, mode),
        26 => divide_const::<26>(magnitude, negative, scale, mode),
        27 => divide_const::<27>(magnitude, negative, scale, mode),
        28 => divide_const::<28>(magnitude, negative, scale, mode),
        29 => divide_const::<29>(magnitude, negative, scale, mode),
        30 => divide_const::<30>(magnitude, negative, scale, mode),
        31 => divide_const::<31>(magnitude, negative, scale, mode),
        32 => divide_const::<32>(magnitude, negative, scale, mode),
        33 => divide_const::<33>(magnitude, negative, scale, mode),
        34 => divide_const::<34>(magnitude, negative, scale, mode),
        35 => divide_const::<35>(magnitude, negative, scale, mode),
        36 => divide_const::<36>(magnitude, negative, scale, mode),
        _ => Err(NumericError::UnsupportedScale),
    }
}
#[inline]
fn divide_const<const K: u32>(
    magnitude: u128,
    negative: bool,
    scale: u8,
    mode: Rounding,
) -> Result<Fixed, NumericError> {
    let divisor = const { 10_u128.pow(K) };
    let quotient = magnitude
        .checked_div(divisor)
        .ok_or(NumericError::Overflow)?;
    let remainder = magnitude
        .checked_rem(divisor)
        .ok_or(NumericError::Overflow)?;
    rounded(quotient, remainder, divisor, negative, scale, mode)
}

fn rounded(
    quotient: u128,
    remainder: u128,
    divisor: u128,
    negative: bool,
    scale: u8,
    rounding: Rounding,
) -> Result<Fixed, NumericError> {
    let limit = if negative {
        i128::MIN.unsigned_abs()
    } else {
        i128::MAX.unsigned_abs()
    };
    if quotient > limit {
        return Err(NumericError::Overflow);
    }
    let increment = if remainder == 0 {
        false
    } else {
        match rounding {
            Rounding::Exact => return Err(NumericError::RoundingRequired),
            Rounding::Down => false,
            Rounding::HalfEven | Rounding::HalfUp => {
                let other_half = divisor
                    .checked_sub(remainder)
                    .ok_or(NumericError::Overflow)?;
                remainder > other_half
                    || (remainder == other_half
                        && (rounding == Rounding::HalfUp || quotient & 1 == 1))
            }
        }
    };
    let magnitude = quotient
        .checked_add(u128::from(increment))
        .ok_or(NumericError::Overflow)?;
    let raw = if negative {
        if magnitude == i128::MIN.unsigned_abs() {
            i128::MIN
        } else {
            i128::try_from(magnitude)
                .map_err(|_| NumericError::Overflow)?
                .checked_neg()
                .ok_or(NumericError::Overflow)?
        }
    } else {
        i128::try_from(magnitude).map_err(|_| NumericError::Overflow)?
    };
    Fixed::new(raw, scale)
}

// SAFETY-BOUNDS: the four 64x64 products fit u128; the middle sum has at most
// 66 bits. The high sum is precisely the high half of a 256-bit product.
#[allow(clippy::arithmetic_side_effects)]
fn wide_product(left: u128, right: u128) -> (u128, u128) {
    let mask = u128::from(u64::MAX);
    let low = (left & mask) * (right & mask);
    let cross_a = (left >> 64) * (right & mask);
    let cross_b = (left & mask) * (right >> 64);
    let high = (left >> 64) * (right >> 64);
    let middle = (low >> 64) + (cross_a & mask) + (cross_b & mask);
    (
        high + (cross_a >> 64) + (cross_b >> 64) + (middle >> 64),
        (low & mask) | ((middle & mask) << 64),
    )
}

#[cold]
fn wide_mul_rescale(
    left: Fixed,
    right: Fixed,
    scale: u8,
    rounding: Rounding,
) -> Result<Fixed, NumericError> {
    let from = left
        .scale
        .checked_add(right.scale)
        .ok_or(NumericError::Overflow)?;
    if scale >= from {
        return Err(NumericError::Overflow);
    }
    let divisor = 10_u128
        .checked_pow(u32::from(from.wrapping_sub(scale)))
        .ok_or(NumericError::Overflow)?;
    let (hi, lo) = wide_product(left.raw.unsigned_abs(), right.raw.unsigned_abs());
    let (quotient_hi, quotient, remainder) = wide_div(hi, lo, divisor);
    if quotient_hi != 0 {
        return Err(NumericError::Overflow);
    }
    rounded(
        quotient,
        remainder,
        divisor,
        (left.raw < 0) != (right.raw < 0),
        scale,
        rounding,
    )
}

#[cold]
fn wide_mul_add(
    left: Fixed,
    right: Fixed,
    addend: Fixed,
    scale: u8,
    mode: Rounding,
) -> Result<Fixed, NumericError> {
    let product_scale = left
        .scale
        .checked_add(right.scale)
        .ok_or(NumericError::Overflow)?;
    let common = product_scale.max(addend.scale).max(scale);
    let product_factor = 10_u128
        .checked_pow(u32::from(common.wrapping_sub(product_scale)))
        .ok_or(NumericError::Overflow)?;
    let add_factor = 10_u128
        .checked_pow(u32::from(common.wrapping_sub(addend.scale)))
        .ok_or(NumericError::Overflow)?;
    let product = wide_scale(
        wide_product(left.raw.unsigned_abs(), right.raw.unsigned_abs()),
        product_factor,
    )?;
    let extra = wide_product(addend.raw.unsigned_abs(), add_factor);
    let product_negative = (left.raw < 0) != (right.raw < 0);
    let extra_negative = addend.raw < 0;
    let (magnitude, negative) = if product_negative == extra_negative {
        (wide_add(product, extra)?, product_negative)
    } else if product >= extra {
        (wide_sub(product, extra)?, product_negative)
    } else {
        (wide_sub(extra, product)?, extra_negative)
    };
    let divisor = 10_u128
        .checked_pow(u32::from(common.wrapping_sub(scale)))
        .ok_or(NumericError::Overflow)?;
    let (high, quotient, remainder) = wide_div(magnitude.0, magnitude.1, divisor);
    if high != 0 {
        return Err(NumericError::Overflow);
    }
    rounded(quotient, remainder, divisor, negative, scale, mode)
}
fn wide_scale(value: (u128, u128), factor: u128) -> Result<(u128, u128), NumericError> {
    let (carry, low) = wide_product(value.1, factor);
    let high = value
        .0
        .checked_mul(factor)
        .and_then(|v| v.checked_add(carry))
        .ok_or(NumericError::Overflow)?;
    Ok((high, low))
}
fn wide_add(left: (u128, u128), right: (u128, u128)) -> Result<(u128, u128), NumericError> {
    let (low, carry) = left.1.overflowing_add(right.1);
    let high = left
        .0
        .checked_add(right.0)
        .and_then(|v| v.checked_add(u128::from(carry)))
        .ok_or(NumericError::Overflow)?;
    Ok((high, low))
}
fn wide_sub(left: (u128, u128), right: (u128, u128)) -> Result<(u128, u128), NumericError> {
    let (low, borrow) = left.1.overflowing_sub(right.1);
    let high = left
        .0
        .checked_sub(right.0)
        .and_then(|v| v.checked_sub(u128::from(borrow)))
        .ok_or(NumericError::Overflow)?;
    Ok((high, low))
}

// SAFETY-BOUNDS: shifts are 0..127; divisor <= 10^36, so remainder < 2^120
// and doubling it cannot overflow u128. No full-width division is required.
#[allow(clippy::arithmetic_side_effects)]
fn wide_div(hi: u128, lo: u128, divisor: u128) -> (u128, u128, u128) {
    let mut remainder = 0_u128;
    let mut quotient_hi = 0_u128;
    let mut quotient_lo = 0_u128;
    for bit in (0..256_u32).rev() {
        let incoming = if bit >= 128 {
            (hi >> (bit - 128)) & 1
        } else {
            (lo >> bit) & 1
        };
        remainder = (remainder << 1) | incoming;
        if remainder >= divisor {
            remainder -= divisor;
            if bit >= 128 {
                quotient_hi |= 1_u128 << (bit - 128);
            } else {
                quotient_lo |= 1_u128 << bit;
            }
        }
    }
    (quotient_hi, quotient_lo, remainder)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    #[test]
    fn req_num_001_decimal_grammar_and_per_value_scale() {
        for invalid in ["", "-", "+1", ".5", "1.", "1e2", "NaN", "1..0", " 1", "1 "] {
            assert!(invalid.parse::<Fixed>().is_err(), "{invalid:?}");
        }
        assert_eq!(
            Fixed::parse("-123.4500", 4).unwrap().to_string(),
            "-123.4500"
        );
        assert_eq!(
            Fixed::parse("1.001", 2),
            Err(NumericError::UnsupportedScale)
        );
        assert_eq!(
            Fixed::parse("0.0000000000000000001", 18),
            Err(NumericError::UnsupportedScale)
        );
    }

    #[test]
    fn req_num_002_signed_limits_and_checked_overflow() {
        for raw in [i128::MIN, i128::MAX, -1, 0, 1] {
            for scale in 0..=18 {
                let value = Fixed::new(raw, scale).unwrap();
                assert_eq!(value.to_string().parse::<Fixed>().unwrap(), value);
            }
        }
        let max = Fixed::new(i128::MAX, 0).unwrap();
        assert_eq!(
            max.checked_add(Fixed::new(1, 0).unwrap()),
            Err(NumericError::Overflow)
        );
        assert_eq!(
            max.mul_rescale(max, 0, Rounding::Exact),
            Err(NumericError::Overflow)
        );
        assert_eq!(
            "170141183460469231731687303715884105728".parse::<Fixed>(),
            Err(NumericError::Overflow)
        );
    }

    #[test]
    fn req_num_003_all_rounding_modes_signed_ties() {
        for (input, even, up, down) in [
            ("1.225", "1.22", "1.23", "1.22"),
            ("1.235", "1.24", "1.24", "1.23"),
            ("-1.225", "-1.22", "-1.23", "-1.22"),
            ("-1.235", "-1.24", "-1.24", "-1.23"),
        ] {
            let value: Fixed = input.parse().unwrap();
            assert_eq!(
                value.rescale(2, Rounding::HalfEven).unwrap().to_string(),
                even
            );
            assert_eq!(value.rescale(2, Rounding::HalfUp).unwrap().to_string(), up);
            assert_eq!(value.rescale(2, Rounding::Down).unwrap().to_string(), down);
            assert_eq!(
                value.rescale(2, Rounding::Exact),
                Err(NumericError::RoundingRequired)
            );
        }
    }

    #[test]
    fn req_num_004_wide_intermediate_can_fit_final_value() {
        let a: Fixed = "10000000000000000000.000000000000000000".parse().unwrap();
        let b: Fixed = "1.000000000000000000".parse().unwrap();
        assert!(a.raw.checked_mul(b.raw).is_none());
        assert_eq!(a.mul_rescale(b, 18, Rounding::Exact).unwrap(), a);
        let min = Fixed::new(i128::MIN, 18).unwrap();
        assert_eq!(min.mul_rescale(b, 18, Rounding::Exact).unwrap(), min);
        let larger_than_one: Fixed = "1.000000000000000001".parse().unwrap();
        assert_eq!(
            min.mul_rescale(larger_than_one, 18, Rounding::Exact),
            Err(NumericError::Overflow)
        );
    }

    #[test]
    fn req_num_005_compare_by_upscaling_without_overflow() {
        assert_eq!(
            "1.00"
                .parse::<Fixed>()
                .unwrap()
                .numeric_cmp("1".parse().unwrap()),
            Ordering::Equal
        );
        assert_eq!(
            Fixed::new(i128::MAX, 0)
                .unwrap()
                .numeric_cmp(Fixed::new(i128::MAX, 18).unwrap()),
            Ordering::Greater
        );
        assert_eq!(
            Fixed::new(i128::MIN, 0)
                .unwrap()
                .numeric_cmp(Fixed::new(i128::MIN, 18).unwrap()),
            Ordering::Less
        );
    }

    #[test]
    fn req_num_006_roundtrip_100k_values() {
        let mut seed = 0x1234_5678_9abc_def0_u128;
        for index in 0..100_000_u32 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let raw = i128::from_le_bytes(seed.to_le_bytes());
            let scale = u8::try_from(index % 19).unwrap();
            let value = Fixed::new(raw, scale).unwrap();
            assert_eq!(value.to_string().parse::<Fixed>().unwrap(), value);
        }
    }

    #[test]
    fn req_num_002_checked_operations_and_parser_never_panic() {
        let mut seed = 0xa22b_05ee_1f3b_7a21_u128;
        for index in 0..100_000_u32 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let left = Fixed::new(
                i128::from_le_bytes(seed.to_le_bytes()),
                u8::try_from(index % 19).unwrap(),
            )
            .unwrap();
            let right = Fixed::new(
                i128::from_le_bytes(seed.rotate_left(53).to_le_bytes()),
                u8::try_from((index / 19) % 19).unwrap(),
            )
            .unwrap();
            let _ = left.checked_add(right);
            let _ = left.checked_sub(right);
            let _ = left.numeric_cmp(right);
            let text: String = seed
                .to_le_bytes()
                .iter()
                .map(|byte| char::from(*byte))
                .collect();
            let _ = text.parse::<Fixed>();
        }
    }
}
