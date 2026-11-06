//! K01–K04: exact arithmetic and independent integer reference models.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
use super::{Fixed, NumericError, Rounding};
fn fixed() -> Fixed {
    let raw = kani::any();
    let scale: u8 = kani::any();
    kani::assume(scale <= 18);
    Fixed::new(raw, scale).unwrap()
}
// Split the finite scale domain into separate solver queries; operand widths
// remain i128. Together these harnesses cover all 19 * 19 scale combinations.
fn add_at_scales<const LEFT: u8, const RIGHT: u8>() {
    let left_raw: i128 = kani::any();
    let right_raw: i128 = kani::any();
    let scale = LEFT.max(RIGHT);
    let expected = left_raw
        .checked_mul(10_i128.pow(u32::from(scale - LEFT)))
        .zip(right_raw.checked_mul(10_i128.pow(u32::from(scale - RIGHT))))
        .and_then(|(a, b)| a.checked_add(b));
    assert_eq!(
        Fixed::new(left_raw, LEFT)
            .unwrap()
            .checked_add(Fixed::new(right_raw, RIGHT).unwrap())
            .map(|v| (v.raw(), v.scale())),
        expected
            .map(|raw| (raw, scale))
            .ok_or(NumericError::Overflow)
    );
}
macro_rules! add_right_scales {
    ($left:literal; $($name:ident = $right:literal),+ $(,)?) => {
        $(
            #[kani::proof]
            #[kani::unwind(40)]
            fn $name() { super::add_at_scales::<$left, $right>(); }
        )+
    };
}
macro_rules! add_left_scale {
    ($name:ident = $left:literal) => {
        mod $name {
            add_right_scales!($left;
                r00=0, r01=1, r02=2, r03=3, r04=4, r05=5, r06=6,
                r07=7, r08=8, r09=9, r10=10, r11=11, r12=12,
                r13=13, r14=14, r15=15, r16=16, r17=17, r18=18);
        }
    };
}
add_left_scale!(fixed_checked_add_l00 = 0);
add_left_scale!(fixed_checked_add_l01 = 1);
add_left_scale!(fixed_checked_add_l02 = 2);
add_left_scale!(fixed_checked_add_l03 = 3);
add_left_scale!(fixed_checked_add_l04 = 4);
add_left_scale!(fixed_checked_add_l05 = 5);
add_left_scale!(fixed_checked_add_l06 = 6);
add_left_scale!(fixed_checked_add_l07 = 7);
add_left_scale!(fixed_checked_add_l08 = 8);
add_left_scale!(fixed_checked_add_l09 = 9);
add_left_scale!(fixed_checked_add_l10 = 10);
add_left_scale!(fixed_checked_add_l11 = 11);
add_left_scale!(fixed_checked_add_l12 = 12);
add_left_scale!(fixed_checked_add_l13 = 13);
add_left_scale!(fixed_checked_add_l14 = 14);
add_left_scale!(fixed_checked_add_l15 = 15);
add_left_scale!(fixed_checked_add_l16 = 16);
add_left_scale!(fixed_checked_add_l17 = 17);
add_left_scale!(fixed_checked_add_l18 = 18);

// Base-2^32 multiplication/division is independent of the product's base-2^64
// U256 implementation. The harness covers signed 80-bit operands and scale 0..18.
fn reference_mul(left: Fixed, right: Fixed, to: u8, mode: Rounding) -> Result<Fixed, NumericError> {
    let a = left.raw().unsigned_abs();
    let b = right.raw().unsigned_abs();
    let parts = |value: u128| {
        std::array::from_fn::<u32, 4, _>(|i| {
            u32::try_from((value >> (i * 32)) & u128::from(u32::MAX)).unwrap()
        })
    };
    let (a, b) = (parts(a), parts(b));
    let mut words = [0_u32; 8];
    let mut carry = 0_u128;
    for (i, word) in words.iter_mut().enumerate() {
        let mut sum = carry;
        for j in 0..4 {
            if i >= j && i - j < 4 {
                sum += u128::from(a[j]) * u128::from(b[i - j]);
            }
        }
        *word = u32::try_from(sum & u128::from(u32::MAX)).unwrap();
        carry = sum >> 32;
    }
    assert_eq!(carry, 0);
    let from = left.scale() + right.scale();
    let mut remainder = 0_u128;
    let mut divisor = 1_u128;
    if from > to {
        for _ in 0..from - to {
            let mut digit = 0_u64;
            for word in words.iter_mut().rev() {
                let value = (digit << 32) | u64::from(*word);
                *word = u32::try_from(value / 10).unwrap();
                digit = value % 10;
            }
            remainder += u128::from(digit) * divisor;
            divisor *= 10;
        }
    } else {
        for _ in 0..to - from {
            let mut carry = 0_u64;
            for word in &mut words {
                let value = u64::from(*word) * 10 + carry;
                *word = u32::try_from(value & u64::from(u32::MAX)).unwrap();
                carry = value >> 32;
            }
            assert_eq!(carry, 0);
        }
    }
    if words[4..].iter().any(|word| *word != 0) {
        return Err(NumericError::Overflow);
    }
    let mut quotient = 0_u128;
    for (i, word) in words[..4].iter().enumerate() {
        quotient |= u128::from(*word) << (i * 32);
    }
    let negative = (left.raw() < 0) != (right.raw() < 0);
    let limit = if negative {
        1_u128 << 127
    } else {
        u128::try_from(i128::MAX).unwrap()
    };
    if quotient > limit {
        return Err(NumericError::Overflow);
    }
    if remainder != 0 {
        if mode == Rounding::Exact {
            return Err(NumericError::RoundingRequired);
        }
        let up = match mode {
            Rounding::HalfUp => 2 * remainder >= divisor,
            Rounding::HalfEven => {
                2 * remainder > divisor || 2 * remainder == divisor && quotient % 2 == 1
            }
            _ => false,
        };
        if up {
            quotient += 1;
        }
    }
    if quotient > limit {
        return Err(NumericError::Overflow);
    }
    let raw = if negative && quotient == 1_u128 << 127 {
        i128::MIN
    } else {
        let raw = i128::try_from(quotient).unwrap();
        if negative { -raw } else { raw }
    };
    Fixed::new(raw, to)
}
#[kani::proof]
#[kani::unwind(260)]
fn fixed_checked_mul_rescale() {
    let left = fixed();
    let right = fixed();
    kani::assume(left.raw().unsigned_abs() < 1_u128 << 80);
    kani::assume(right.raw().unsigned_abs() < 1_u128 << 80);
    let to: u8 = kani::any();
    kani::assume(to <= 18);
    let mode = match kani::any::<u8>() % 4 {
        0 => Rounding::Exact,
        1 => Rounding::HalfEven,
        2 => Rounding::HalfUp,
        _ => Rounding::Down,
    };
    assert_eq!(
        left.mul_rescale(right, to, mode),
        reference_mul(left, right, to, mode)
    );
}
#[kani::proof]
#[kani::unwind(40)]
fn fixed_rescale_roundtrip() {
    let value = fixed();
    let up: u8 = kani::any();
    kani::assume(up >= value.scale() && up <= 18);
    if let Ok(scaled) = value.rescale(up, Rounding::Exact) {
        assert_eq!(scaled.rescale(value.scale(), Rounding::Exact), Ok(value));
    }
}
fn reference_cmp(left: Fixed, right: Fixed) -> std::cmp::Ordering {
    fn lower(left: i128, right: i128, difference: u8) -> std::cmp::Ordering {
        let divisor = 10_i128.pow(u32::from(difference));
        left.cmp(&(right / divisor))
            .then_with(|| 0.cmp(&(right % divisor)))
    }
    if left.scale() <= right.scale() {
        lower(left.raw(), right.raw(), right.scale() - left.scale())
    } else {
        lower(right.raw(), left.raw(), left.scale() - right.scale()).reverse()
    }
}
#[kani::proof]
#[kani::unwind(40)]
fn fixed_cmp_total_order() {
    let left = fixed();
    let right = fixed();
    assert_eq!(left.numeric_cmp(right), reference_cmp(left, right));
    assert_eq!(left.numeric_cmp(right), right.numeric_cmp(left).reverse());
}
