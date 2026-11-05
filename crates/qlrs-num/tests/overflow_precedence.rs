//! Both error conditions hold; the established contract reports final overflow.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_num::{Fixed, NumericError, Rounding};
#[test]
fn hard_004_overflow_precedes_inexact_rounding() {
    let left = Fixed::new(i128::MIN, 0).unwrap();
    let right = Fixed::new(11, 1).unwrap();
    assert_eq!(
        left.mul_rescale(right, 0, Rounding::Exact),
        Err(NumericError::Overflow)
    );
}
