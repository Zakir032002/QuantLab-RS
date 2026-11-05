//! Minimized native/cargo-fuzz replay of an incorrect generated-anchor request.
#![forbid(unsafe_code)]
#[test]
fn fuzz_001_minimized_s8_overfill_anchor() {
    qlrs_testkit::oracles::reduce_sound(include_bytes!(
        "../../../tests/regressions/fuzz/reduce_sound/P3-ORACLE-001"
    ));
}
