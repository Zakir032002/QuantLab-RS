//! Measured million-event Gate B-synthetic; emits raw evidence and timings.
#![forbid(unsafe_code)]
use std::time::Instant;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for defect in [
        qlrs_testkit::Defect::D01,
        qlrs_testkit::Defect::D03,
        qlrs_testkit::Defect::D07,
    ] {
        let fixture = qlrs_testkit::generate(200_000, 42, Some((defect, 140_000)))?;
        let bytes = fixture.bytes.len();
        let digest = qlrs_io::digest(&fixture.bytes);
        let start = Instant::now();
        let mut input = qlrs_io::ParsedTrace::parse(fixture.bytes)?;
        let parse = start.elapsed();
        let start = Instant::now();
        qlrs_ledger::Ledger::prepare(&mut input.store);
        let prepare = start.elapsed();
        let mask = qlrs_schema::AliveMask::all(input.store.units.len());
        let report = qlrs_check::Checker::check(&input.store, &mask, None, false);
        assert_eq!(report.status, qlrs_schema::Status::Violation);
        let start = Instant::now();
        let reduced = qlrs_reduce::reduce(&input.store, &digest, None, fixture.target_seq)?;
        let elapsed = start.elapsed();
        assert!(reduced.closure_1_minimal);
        assert!(reduced.retained_events <= 6);
        assert!(
            reduced
                .checkpoint
                .as_ref()
                .is_some_and(|c| c.provenance == "synthetic")
        );
        let mut witness = Vec::new();
        input.write_kept(&reduced.alive, &mut witness)?;
        for line in witness.split_inclusive(|b| *b == b'\n') {
            assert!(
                input
                    .source
                    .split_inclusive(|b| *b == b'\n')
                    .any(|original| original == line)
            );
        }
        let mut parsed = qlrs_io::ParsedTrace::parse(witness)?;
        qlrs_ledger::Ledger::prepare(&mut parsed.store);
        let replay = qlrs_check::Checker::check(
            &parsed.store,
            &qlrs_schema::AliveMask::all(parsed.store.units.len()),
            reduced.checkpoint.as_ref(),
            false,
        );
        assert!(
            replay
                .keys()
                .any(|k| Some(k) == reduced.target.instance_key.as_ref())
        );
        println!(
            "defect={defect:?} seed=42 events={} source_bytes={bytes} source_blake3={digest} column_bytes={} parse_ns={} prepare_ns={} reduction_ns={} kept_events={} predicate_calls={} closure_1_minimal=true verbatim=true target_replayed=true",
            input.store.len(),
            input.store.column_bytes(),
            parse.as_nanos(),
            prepare.as_nanos(),
            elapsed.as_nanos(),
            reduced.retained_events,
            reduced.log.len()
        );
    }
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines().filter(|line| line.starts_with("VmHWM:")) {
            println!("{line}");
        }
    }
    Ok(())
}
