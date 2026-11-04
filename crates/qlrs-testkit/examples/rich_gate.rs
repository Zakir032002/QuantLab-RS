//! Extended D01-D11 gate with explicit input size, event and reduction budgets.
#![forbid(unsafe_code)]
use std::time::{Duration, Instant};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let orders = args
        .first()
        .map_or(Ok(250_000_usize), |value| value.parse())?;
    let kept_limit = args.get(1).map_or(Ok(128_usize), |value| value.parse())?;
    let time_limit_ms = args.get(2).map_or(Ok(2000_u64), |value| value.parse())?;
    for profile in ["interleaved", "long-lived", "sparse"] {
        let config = qlrs_testkit::GeneratorConfig::profile(profile, orders, 42)?;
        let clean = qlrs_testkit::generate_rich(config.clone(), None)?;
        let mut parsed = qlrs_io::ParsedTrace::parse(clean.bytes)?;
        qlrs_ledger::Ledger::prepare(&mut parsed.store);
        let clean_report = qlrs_check::Checker::check(
            &parsed.store,
            &qlrs_schema::AliveMask::all(parsed.store.units.len()),
            None,
            false,
        );
        assert_eq!(clean_report.status, qlrs_schema::Status::Consistent);
        println!(
            "profile={profile} clean=true seed=42 events={} source_blake3={} status=CONSISTENT",
            parsed.store.len(),
            qlrs_io::digest(&parsed.source)
        );
        drop(parsed);
        for defect in qlrs_testkit::DEFECTS.into_iter().take(11) {
            let index = if profile == "long-lived" {
                0
            } else {
                orders / 2
            };
            let fixture = qlrs_testkit::generate_rich(config.clone(), Some((defect, index)))?;
            let digest = qlrs_io::digest(&fixture.bytes);
            let bytes = fixture.bytes.len();
            let parse_start = Instant::now();
            let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes)?;
            let parse = parse_start.elapsed();
            qlrs_ledger::Ledger::prepare(&mut trace.store);
            let start = Instant::now();
            let reduced = qlrs_reduce::reduce_with_limits(
                &trace.store,
                &digest,
                None,
                fixture.target_seq,
                qlrs_reduce::ReductionLimits {
                    max_calls: 1024,
                    time_limit: Duration::from_millis(time_limit_ms),
                },
            )?;
            let elapsed = start.elapsed();
            assert!(
                reduced.closure_1_minimal,
                "{profile} {defect:?} {:?}",
                reduced.stop_reason
            );
            assert!(
                elapsed <= Duration::from_millis(time_limit_ms),
                "{profile} {defect:?} exceeded time gate"
            );
            assert!(
                reduced.retained_events <= kept_limit,
                "{profile} {defect:?} retained {}",
                reduced.retained_events
            );
            let mut witness = Vec::new();
            trace.write_kept(&reduced.alive, &mut witness)?;
            for line in witness.split_inclusive(|byte| *byte == b'\n') {
                assert!(
                    trace
                        .source
                        .split_inclusive(|byte| *byte == b'\n')
                        .any(|source| source == line)
                );
            }
            let mut replay = qlrs_io::ParsedTrace::parse(witness)?;
            qlrs_ledger::Ledger::prepare_with_checkpoint(
                &mut replay.store,
                reduced.checkpoint.as_ref(),
            )?;
            let result = qlrs_check::Checker::check(
                &replay.store,
                &qlrs_schema::AliveMask::all(replay.store.units.len()),
                reduced.checkpoint.as_ref(),
                false,
            );
            assert_eq!(result.status, qlrs_schema::Status::Violation);
            assert!(
                result
                    .keys()
                    .any(|key| Some(key) == reduced.target.instance_key.as_ref())
            );
            let checkpoint = reduced
                .checkpoint
                .as_ref()
                .ok_or("missing synthetic checkpoint")?;
            assert_eq!(checkpoint.provenance, "synthetic");
            let log_bytes = reduced
                .log
                .iter()
                .map(|entry| qlrs_io::encode(entry).map(|bytes| bytes.len()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .sum::<usize>();
            println!(
                "profile={profile} defect={defect:?} seed=42 index={index} events={} source_bytes={bytes} source_blake3={digest} parse_ns={} reduction_ns={} kept_events={} kept_limit={kept_limit} time_limit_ms={time_limit_ms} predicate_calls={} log_bytes={log_bytes} cut_seq={} checkpoint_orders={} closure_1_minimal=true verbatim=true target_replayed=true",
                trace.store.len(),
                parse.as_nanos(),
                elapsed.as_nanos(),
                reduced.retained_events,
                reduced.predicate_calls,
                checkpoint.cut_seq,
                checkpoint.orders.len()
            );
        }
    }
    println!("rich_gate=PASS profiles=3 violating_defects_per_profile=11");
    Ok(())
}
