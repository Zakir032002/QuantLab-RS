//! Canonical encoding may expand a record that fits the input byte limit.
#![forbid(unsafe_code)]
#[test]
fn fuzz_002_canonical_expansion_at_line_limit() {
    let input = include_bytes!(
        "../../../tests/regressions/fuzz/io_line/P3-ORACLE-002"
    );
    let limits = qlrs_io::Limits {
        max_line_bytes: input.len(),
        ..qlrs_io::Limits::default()
    };
    let canonical = qlrs_io::canonical_record(input, limits).expect("valid capped input");
    assert!(canonical.len() > input.len());
    assert_eq!(
        qlrs_io::canonical_record(&canonical, limits)
            .expect_err("the original input cap remains enforced")
            .code,
        "E-LIMIT-LINE-BYTES"
    );
    qlrs_testkit::oracles::io_line(input);
}
