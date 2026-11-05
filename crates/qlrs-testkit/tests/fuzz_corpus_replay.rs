//! Re-run committed fuzz discoveries with the stable compiler and normal tests.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
fn replay(target: &str, oracle: fn(&[u8])) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in [
        root.join("fuzz/corpus").join(target),
        root.join("tests/regressions/fuzz").join(target),
    ] {
        if dir.exists() {
            files.extend(
                std::fs::read_dir(dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|p| p.is_file()),
            );
        }
    }
    files.sort();
    assert!(!files.is_empty(), "missing seed corpus: {target}");
    for path in files {
        oracle(&std::fs::read(path).unwrap());
    }
}
#[test]
fn fuzz_001_io_line_corpus() {
    replay("io_line", qlrs_testkit::oracles::io_line);
}
#[test]
fn fuzz_001_io_trace_corpus() {
    replay("io_trace", qlrs_testkit::oracles::io_trace);
}
#[test]
fn fuzz_001_num_ops_corpus() {
    replay("num_ops", qlrs_testkit::oracles::num_ops);
}
#[test]
fn fuzz_001_num_decimal_corpus() {
    replay("num_decimal", qlrs_testkit::oracles::num_decimal);
}

#[test]
fn fuzz_001_io_chunk_diff_corpus() {
    replay("io_chunk_diff", qlrs_testkit::oracles::io_chunk_diff);
}

#[test]
fn fuzz_001_check_clean_corpus() {
    replay("check_clean", qlrs_testkit::oracles::check_clean);
}

#[test]
fn fuzz_001_check_injected_corpus() {
    replay("check_injected", qlrs_testkit::oracles::check_injected);
}

#[test]
fn fuzz_001_check_dense_vs_selected_corpus() {
    replay(
        "check_dense_vs_selected",
        qlrs_testkit::oracles::check_dense_vs_selected,
    );
}

#[test]
fn fuzz_001_reduce_sound_corpus() {
    replay("reduce_sound", qlrs_testkit::oracles::reduce_sound);
}

#[test]
fn fuzz_001_eval_vs_checker_corpus() {
    replay("eval_vs_checker", qlrs_testkit::oracles::eval_vs_checker);
}

#[test]
fn fuzz_001_bundle_verify_corpus() {
    replay("bundle_verify", qlrs_testkit::oracles::bundle_verify);
}
