//! End-to-end CLI status codes, error lines and offline bundle replay.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;
#[test]
// One end-to-end flow owns and validates the same temporary evidence directory.
#[allow(clippy::too_many_lines)]
fn req_cli_001_exit_codes_and_bundle_commands() {
    let temp = std::env::temp_dir().join(format!("qlrs-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let trace = temp.join("trace.jsonl");
    let cli = env!("CARGO_BIN_EXE_qlrs");
    for (defect, code) in [
        (None, 0),
        (Some(qlrs_testkit::Defect::D03), 2),
        (Some(qlrs_testkit::Defect::D14), 3),
        (Some(qlrs_testkit::Defect::D12), 4),
    ] {
        let fixture = qlrs_testkit::generate(3, 0, defect.map(|d| (d, 1))).unwrap();
        std::fs::write(&trace, fixture.bytes).unwrap();
        let result = Command::new(cli)
            .args(["verify"])
            .arg(&trace)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    std::fs::write(&trace, b"{}\n").unwrap();
    let malformed = Command::new(cli)
        .arg("verify")
        .arg(&trace)
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("line 1"));
    let fixture = qlrs_testkit::generate(20, 0, Some((qlrs_testkit::Defect::D03, 10))).unwrap();
    std::fs::write(&trace, fixture.bytes).unwrap();
    let bundle = temp.join("bundle");
    let output = Command::new(cli)
        .arg("reduce")
        .arg(&trace)
        .arg("--output")
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(cli)
            .args(["bundle", "verify"])
            .arg(&bundle)
            .output()
            .unwrap()
            .status
            .success()
    );
    let reduced = Command::new(cli)
        .arg("verify")
        .arg(bundle.join("reduced_trace.jsonl"))
        .arg("--checkpoint")
        .arg(bundle.join("synthetic_checkpoint.json"))
        .output()
        .unwrap();
    assert_eq!(reduced.status.code(), Some(2));
    let again = temp.join("bundle-again");
    let repeated = Command::new(cli)
        .arg("reduce")
        .arg(bundle.join("reduced_trace.jsonl"))
        .arg("--checkpoint")
        .arg(bundle.join("synthetic_checkpoint.json"))
        .arg("--output")
        .arg(&again)
        .output()
        .unwrap();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert!(
        Command::new(cli)
            .args(["bundle", "verify"])
            .arg(&again)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(bundle.join("reduced_trace.jsonl")).unwrap(),
        std::fs::read(again.join("reduced_trace.jsonl")).unwrap()
    );
    assert_eq!(
        std::fs::read(bundle.join("synthetic_checkpoint.json")).unwrap(),
        std::fs::read(again.join("synthetic_checkpoint.json")).unwrap()
    );
    // An existing output directory must never be overwritten.
    assert_eq!(
        Command::new(cli)
            .arg("reduce")
            .arg(&trace)
            .arg("--output")
            .arg(&bundle)
            .output()
            .unwrap()
            .status
            .code(),
        Some(1)
    );
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn req_cli_002_limits_preserve_verified_output_without_minimality_claim() {
    let temp = std::env::temp_dir().join(format!("qlrs-cli-budget-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let trace = temp.join("trace.jsonl");
    std::fs::write(
        &trace,
        qlrs_testkit::generate(20, 0, Some((qlrs_testkit::Defect::D03, 10)))
            .unwrap()
            .bytes,
    )
    .unwrap();
    for (flag, value) in [
        ("--max-calls", "0"),
        ("--max-calls", "1"),
        ("--time-limit-ms", "0"),
    ] {
        let bundle = temp.join(format!("{}-{value}", flag.trim_start_matches('-')));
        let output = Command::new(env!("CARGO_BIN_EXE_qlrs"))
            .arg("reduce")
            .arg(&trace)
            .arg("--output")
            .arg(&bundle)
            .args([flag, value])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest: qlrs_bundle::Manifest = qlrs_io::decode(&output.stdout).unwrap();
        assert!(!manifest.closure_1_minimal);
        assert!(manifest.stop_reason.is_some());
        assert!(
            Command::new(env!("CARGO_BIN_EXE_qlrs"))
                .args(["bundle", "verify"])
                .arg(&bundle)
                .status()
                .unwrap()
                .success()
        );
    }
    std::fs::remove_dir_all(temp).unwrap();
}
