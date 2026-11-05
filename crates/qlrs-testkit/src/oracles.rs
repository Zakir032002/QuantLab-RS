//! Independent integer oracles shared by libFuzzer and stable corpus replay.
use arbitrary::Unstructured;
use num_bigint::{BigInt, Sign};
use qlrs_io::{Limits, ParsedTrace};
use qlrs_num::{Fixed, NumericError, Rounding};

fn limits() -> Limits {
    Limits {
        max_line_bytes: 4096,
        max_events: 2000,
        max_interned: 1024,
        max_input_bytes: 1 << 20,
        ..Limits::default()
    }
}

/// One record canonicalizes idempotently, or produces a stable typed error.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn io_line(data: &[u8]) {
    match qlrs_io::canonical_record(data, limits()) {
        Ok(canonical) => assert_eq!(
            qlrs_io::canonical_record(&canonical, limits()).ok(),
            Some(canonical)
        ),
        Err(error) => assert!(
            error.code.starts_with("E-")
                || error.code.starts_with("U-")
                || error.code.starts_with("I-")
        ),
    }
}
/// Bounded complete traces have valid shapes or an error inside their input.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn io_trace(data: &[u8]) {
    match ParsedTrace::parse_with_limits(data.to_vec(), limits()) {
        Ok(trace) => assert!(trace.store.shape_valid()),
        Err(error) => {
            let lines = data.split_inclusive(|byte| *byte == b'\n').count().max(1);
            assert!(error.line <= lines, "{} > {lines}", error.line);
            assert!(
                error.code.starts_with("E-")
                    || error.code.starts_with("U-")
                    || error.code.starts_with("I-")
            );
        }
    }
}
fn power(scale: u8) -> BigInt {
    BigInt::from(10).pow(u32::from(scale))
}
fn narrow(value: BigInt) -> Result<i128, NumericError> {
    i128::try_from(value).map_err(|_| NumericError::Overflow)
}
fn quantize(value: BigInt, from: u8, to: u8, mode: Rounding) -> Result<i128, NumericError> {
    if from <= to {
        return narrow(value * power(to.saturating_sub(from)));
    }
    let divisor = power(from.saturating_sub(to));
    let mut quotient = &value / &divisor;
    // SPEC's final-overflow priority, also modeled by ci/numeric-oracle.py.
    narrow(quotient.clone())?;
    let remainder = &value % &divisor;
    if remainder != BigInt::from(0) {
        if mode == Rounding::Exact {
            return Err(NumericError::RoundingRequired);
        }
        let magnitude = if remainder.sign() == Sign::Minus {
            -remainder
        } else {
            remainder
        };
        let twice = magnitude * 2;
        let up = match mode {
            Rounding::HalfUp => twice >= divisor,
            Rounding::HalfEven => {
                twice > divisor || (twice == divisor && &quotient % 2 != BigInt::from(0))
            }
            _ => false,
        };
        if up {
            quotient += if value.sign() == Sign::Minus { -1 } else { 1 };
        }
    }
    narrow(quotient)
}
/// Full-width operations agree with big integers and documented alignment bounds.
/// Arbitrary division is not exposed by Fixed; rescale covers its power-of-ten
/// division, and negation uses zero minus the operand at the same scale.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn num_ops(data: &[u8]) {
    let Ok((left, right, ls, rs, target, op, mode)) =
        Unstructured::new(data).arbitrary::<(i128, i128, u8, u8, u8, u8, u8)>()
    else {
        return;
    };
    let (ls, rs, target) = (ls % 19, rs % 19, target % 19);
    let left_fixed = Fixed::new(left, ls).unwrap_or_else(|e| panic!("{e}"));
    let right_fixed = Fixed::new(right, rs).unwrap_or_else(|e| panic!("{e}"));
    let mode = [
        Rounding::Exact,
        Rounding::HalfEven,
        Rounding::HalfUp,
        Rounding::Down,
    ]
    .get(usize::from(mode % 4))
    .copied()
    .unwrap_or(Rounding::Exact);
    let common = ls.max(rs);
    let (actual, expected, scale) = match op % 7 {
        0 | 1 => {
            let aligned_left = BigInt::from(left) * power(common.saturating_sub(ls));
            let aligned_right = BigInt::from(right) * power(common.saturating_sub(rs));
            let expected = if narrow(aligned_left.clone()).is_err()
                || narrow(aligned_right.clone()).is_err()
            {
                Err(NumericError::Overflow)
            } else {
                narrow(if op % 7 == 0 {
                    aligned_left + aligned_right
                } else {
                    aligned_left - aligned_right
                })
            };
            (
                if op % 7 == 0 {
                    left_fixed.checked_add(right_fixed)
                } else {
                    left_fixed.checked_sub(right_fixed)
                },
                expected,
                common,
            )
        }
        2 => (
            left_fixed.checked_mul(right_fixed),
            if ls.saturating_add(rs) > 18 {
                Err(NumericError::UnsupportedScale)
            } else {
                narrow(BigInt::from(left) * BigInt::from(right))
            },
            ls.saturating_add(rs),
        ),
        3 => (
            left_fixed.mul_rescale(right_fixed, target, mode),
            quantize(
                BigInt::from(left) * BigInt::from(right),
                ls.saturating_add(rs),
                target,
                mode,
            ),
            target,
        ),
        4 => (
            left_fixed.rescale(target, mode),
            quantize(BigInt::from(left), ls, target, mode),
            target,
        ),
        5 => {
            assert_eq!(
                left_fixed.numeric_cmp(right_fixed),
                (BigInt::from(left) * power(rs)).cmp(&(BigInt::from(right) * power(ls)))
            );
            return;
        }
        _ => (
            Fixed::new(0, ls).and_then(|zero| zero.checked_sub(left_fixed)),
            narrow(-BigInt::from(left)),
            ls,
        ),
    };
    assert_eq!(
        actual.map(|value| (value.raw(), value.scale())),
        expected.map(|raw| (raw, scale))
    );
}
/// Literal acceptance agrees with integer parsing; formatting accepted literals
/// round-trips their raw value and scale without floating-point conversion.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn num_decimal(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let bounded = limits().number(text, 1);
    if text.len() > 40 {
        assert_eq!(bounded.err().map(|e| e.code), Some("E-LIMIT-NUMBER-LEN"));
        return;
    }
    if text
        .split_once('.')
        .is_some_and(|(_, tail)| tail.len() > 18)
    {
        assert_eq!(bounded.err().map(|e| e.code), Some("E-SCALE"));
        return;
    }
    assert!(bounded.is_ok());
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let syntax = !integer.is_empty()
        && integer.bytes().all(|b| b.is_ascii_digit())
        && fraction.bytes().all(|b| b.is_ascii_digit())
        && (!unsigned.contains('.') || !fraction.is_empty());
    let expected = if syntax && fraction.len() <= 18 {
        BigInt::parse_bytes(format!("{integer}{fraction}").as_bytes(), 10)
            .and_then(|value| {
                i128::try_from(if text.starts_with('-') { -value } else { value }).ok()
            })
            .zip(u8::try_from(fraction.len()).ok())
    } else {
        None
    };
    let actual = Fixed::parse(text, 18);
    assert_eq!(actual.as_ref().ok().map(|v| (v.raw(), v.scale())), expected);
    if let Ok(value) = actual {
        assert_eq!(Fixed::parse(&value.to_string(), 18), Ok(value));
    }
}
