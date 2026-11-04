//! Arbitrary-precision reference checks, independent of the wide implementation.
#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects
)]
use num_bigint::BigInt;
use proptest::prelude::*;
use qlrs_num::{Fixed, NumericError, Rounding};
fn reference(
    a: i128,
    sa: u8,
    b: i128,
    sb: u8,
    target: u8,
    mode: Rounding,
) -> Result<Fixed, NumericError> {
    let product = BigInt::from(a) * BigInt::from(b);
    let negative = product < BigInt::from(0);
    let exponent = i32::from(sa) + i32::from(sb) - i32::from(target);
    let raw = if exponent <= 0 {
        product * BigInt::from(10).pow(exponent.unsigned_abs())
    } else {
        let magnitude = if negative { -product } else { product };
        let divisor = BigInt::from(10).pow(exponent.unsigned_abs());
        let mut quotient = &magnitude / &divisor;
        let remainder = &magnitude % &divisor;
        let limit = BigInt::from(if negative {
            i128::MIN.unsigned_abs()
        } else {
            i128::MAX.unsigned_abs()
        });
        if quotient > limit {
            return Err(NumericError::Overflow);
        }
        if remainder != BigInt::from(0) {
            match mode {
                Rounding::Exact => return Err(NumericError::RoundingRequired),
                Rounding::Down => {}
                Rounding::HalfEven | Rounding::HalfUp => {
                    let doubled = &remainder * BigInt::from(2);
                    if doubled > divisor
                        || (doubled == divisor
                            && (mode == Rounding::HalfUp
                                || &quotient % BigInt::from(2) != BigInt::from(0)))
                    {
                        quotient += 1;
                    }
                }
            }
        }
        if negative { -quotient } else { quotient }
    };
    raw.to_string()
        .parse::<i128>()
        .map_err(|_| NumericError::Overflow)
        .and_then(|raw| Fixed::new(raw, target))
}
fn raw() -> impl Strategy<Value = i128> {
    prop_oneof![
        any::<i128>(),
        -1_000_000_000_000_000_000_000_000_000_i128..1_000_000_000_000_000_000_000_000_000_i128,
        -1_000_000_000_000_000_000_i128..1_000_000_000_000_000_000_i128,
        Just(i128::MIN),
        Just(i128::MAX)
    ]
}
proptest! {
 #![proptest_config(ProptestConfig {cases:100_000,..ProptestConfig::default()})]
 #[test]
 fn req_num_007_mul_rescale_matches_bigint(a in raw(), b in raw(), sa in 0u8..=18,sb in 0u8..=18,target in 0u8..=18){
  let left=Fixed::new(a,sa).unwrap();let right=Fixed::new(b,sb).unwrap();
  for mode in [Rounding::Exact,Rounding::HalfEven,Rounding::HalfUp,Rounding::Down]{prop_assert_eq!(left.mul_rescale(right,target,mode),reference(a,sa,b,sb,target,mode));}
 }
 #[test]
 fn req_num_008_parse_format_roundtrip(raw in raw(),scale in 0u8..=18){let value=Fixed::new(raw,scale).unwrap();prop_assert_eq!(value.to_string().parse::<Fixed>(),Ok(value));}
 #[test]
 fn req_num_009_parser_no_panics(bytes in proptest::collection::vec(any::<u8>(),0..128)){if let Ok(text)=std::str::from_utf8(&bytes){let _=Fixed::parse(text,18);}}
}
#[test]
fn req_num_010_checked_mul_exact_product_scale() {
    let a: Fixed = "2.50".parse().unwrap();
    let b: Fixed = "1.2".parse().unwrap();
    assert_eq!(a.checked_mul(b).unwrap().to_string(), "3.000");
    let wide = Fixed::new(1, 18).unwrap();
    assert_eq!(wide.checked_mul(wide), Err(NumericError::UnsupportedScale));
}
#[test]
fn req_num_011_notional_fee_round_once() {
    let qty: Fixed = "1".parse().unwrap();
    let price: Fixed = "-1.005".parse().unwrap();
    let fee: Fixed = "-0.01".parse().unwrap();
    assert_eq!(
        qty.mul_add_rescale(price, fee, 2, Rounding::HalfEven)
            .unwrap()
            .to_string(),
        "-1.02"
    );
    let max = Fixed::new(i128::MAX, 0).unwrap();
    let neg = Fixed::new(-i128::MAX, 0).unwrap();
    let one = Fixed::new(1, 0).unwrap();
    assert_eq!(
        max.mul_add_rescale(one, neg, 0, Rounding::Exact)
            .unwrap()
            .raw(),
        0
    );
    let wide: Fixed = "10000000000000000000.000000000000000000".parse().unwrap();
    let one: Fixed = "1.000000000000000000".parse().unwrap();
    let add: Fixed = "-0.01".parse().unwrap();
    assert_eq!(
        wide.mul_add_rescale(one, add, 2, Rounding::Exact)
            .unwrap()
            .to_string(),
        "9999999999999999999.99"
    );
}
#[allow(clippy::too_many_arguments)]
fn reference_add(
    a: i128,
    sa: u8,
    b: i128,
    sb: u8,
    c: i128,
    sc: u8,
    target: u8,
    mode: Rounding,
) -> Result<Fixed, NumericError> {
    let common = (u32::from(sa) + u32::from(sb))
        .max(u32::from(sc))
        .max(u32::from(target));
    let product = BigInt::from(a)
        * BigInt::from(b)
        * BigInt::from(10).pow(common - u32::from(sa) - u32::from(sb))
        + BigInt::from(c) * BigInt::from(10).pow(common - u32::from(sc));
    let negative = product < BigInt::from(0);
    let magnitude = if negative { -product } else { product };
    let divisor = BigInt::from(10).pow(common - u32::from(target));
    let mut quotient = &magnitude / &divisor;
    let rem = &magnitude % &divisor;
    if quotient
        > BigInt::from(if negative {
            i128::MIN.unsigned_abs()
        } else {
            i128::MAX.unsigned_abs()
        })
    {
        return Err(NumericError::Overflow);
    }
    if rem != BigInt::from(0) {
        match mode {
            Rounding::Exact => return Err(NumericError::RoundingRequired),
            Rounding::Down => {}
            Rounding::HalfEven | Rounding::HalfUp => {
                if &rem * 2 > divisor
                    || (&rem * 2 == divisor
                        && (mode == Rounding::HalfUp || &quotient % 2 != BigInt::from(0)))
                {
                    quotient += 1;
                }
            }
        }
    }
    let signed = if negative { -quotient } else { quotient };
    signed
        .to_string()
        .parse::<i128>()
        .map_err(|_| NumericError::Overflow)
        .and_then(|raw| Fixed::new(raw, target))
}
proptest! {
 #![proptest_config(ProptestConfig {cases:100_000,..ProptestConfig::default()})]
 #[test]
 fn req_num_012_product_plus_fee_bigint_oracle(a in raw(),b in raw(),c in raw(),sa in 0u8..=18,sb in 0u8..=18,sc in 0u8..=18,target in 0u8..=18){
  for mode in [Rounding::Exact,Rounding::HalfEven,Rounding::HalfUp,Rounding::Down]{prop_assert_eq!(Fixed::new(a,sa).unwrap().mul_add_rescale(Fixed::new(b,sb).unwrap(),Fixed::new(c,sc).unwrap(),target,mode),reference_add(a,sa,b,sb,c,sc,target,mode));}
 }
}
#[test]
fn req_num_013_error_reason_codes() {
    for (error, code) in [
        (NumericError::InvalidDecimal, "invalid-decimal"),
        (NumericError::UnsupportedScale, "unsupported-scale"),
        (NumericError::Overflow, "numeric-overflow"),
        (NumericError::RoundingRequired, "rounding-policy-required"),
    ] {
        assert_eq!(error.to_string(), code);
    }
}
