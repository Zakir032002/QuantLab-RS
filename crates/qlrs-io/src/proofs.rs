//! K05 lives at the resource boundary rather than adding an I/O dependency to num.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
#[kani::proof]
#[kani::unwind(12)]
fn decimal_parse_bounds() {
    let bytes: [u8; 8] = kani::any();
    let length: usize = kani::any();
    kani::assume(length <= 8);
    let maximum: usize = kani::any();
    kani::assume(maximum <= 8);
    if let Ok(text) = std::str::from_utf8(&bytes[..length]) {
        let limits = crate::Limits {
            max_numeric_literal_len: maximum,
            ..crate::Limits::default()
        };
        let bounded = limits.number(text, 1);
        assert_eq!(bounded.is_ok(), length <= maximum);
        if bounded.is_ok() {
            if let Ok(value) = qlrs_num::Fixed::parse(text, 18) {
                assert!(value.scale() <= 18);
            }
        }
    }
}
