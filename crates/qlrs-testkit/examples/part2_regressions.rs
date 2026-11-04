//! Explicit R1 before/after-sized cells, plus a required sparse window over 32.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_testkit::{Defect, GeneratorConfig, generate, generate_rich, generate_shape};
use std::time::{Duration, Instant};
fn main() {
    let cases = [
        ("R1a", generate(400, 42, Some((Defect::D04, 200))).unwrap()),
        (
            "R1b",
            generate(2000, 42, Some((Defect::D04, 1000))).unwrap(),
        ),
        (
            "R1c",
            generate_rich(
                GeneratorConfig::profile("long-lived", 2600, 42).unwrap(),
                Some((Defect::D07, 0)),
            )
            .unwrap(),
        ),
        (
            "R1d",
            generate_shape("S6", 42, 10_000, Some(Defect::D04)).unwrap(),
        ),
        (
            "sparse-over-32",
            generate_shape("S4", 45, 10_000, Some(Defect::D03)).unwrap(),
        ),
    ];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    for (name, fixture) in cases {
        let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
        qlrs_ledger::Ledger::prepare(&mut trace.store);
        let digest = qlrs_io::digest(&trace.source);
        let start = Instant::now();
        let result = pool
            .install(|| {
                qlrs_reduce::reduce_with_limits(
                    &trace.store,
                    &digest,
                    None,
                    fixture.target_seq,
                    qlrs_reduce::ReductionLimits {
                        max_calls: 200,
                        time_limit: Duration::from_secs(5),
                    },
                )
            })
            .unwrap();
        let ns = start.elapsed().as_nanos();
        assert!(result.closure_1_minimal);
        assert!(result.predicate_calls <= 200);
        println!(
            "{{\"case\":\"{name}\",\"actual_events\":{},\"target_seq\":{},\"kept_events\":{},\"candidate_calls\":{},\"full_slots\":{},\"certificate_candidates_tested\":{},\"wall_ns_8\":{ns}}}",
            trace.store.len(),
            fixture.target_seq.unwrap(),
            result.retained_events,
            result.predicate_calls,
            result.full_calls,
            result.certificate.candidates_tested
        );
        if name == "sparse-over-32" {
            assert!(result.retained_events > 32);
        }
    }
}
