//! Canonical encoding may expand a record that fits the input byte limit.
#![forbid(unsafe_code)]
#[test]
#[ignore = "P3-ORACLE-002: canonical round-trip incorrectly reapplies the input size cap"]
fn fuzz_002_canonical_expansion_at_line_limit() {
    qlrs_testkit::oracles::io_line(include_bytes!(
        "../../../tests/regressions/fuzz/io_line/P3-ORACLE-002"
    ));
}
