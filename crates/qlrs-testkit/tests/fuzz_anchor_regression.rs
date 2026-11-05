//! Minimized native/cargo-fuzz replay of an incorrect generated-anchor request.
#![forbid(unsafe_code)]
#[test]
#[ignore = "P3-ORACLE-001: fixture's later anchor has two inseparable I3 breaches; use the reducer's earliest-target contract"]
fn fuzz_001_minimized_s8_overfill_anchor() {
    qlrs_testkit::oracles::reduce_sound(include_bytes!("../../../tests/regressions/fuzz/reduce_sound/P3-ORACLE-001"));
}
