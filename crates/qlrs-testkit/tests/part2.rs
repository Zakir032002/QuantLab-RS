//! Rebased Part 2 regressions; existing Stage A coverage stays active.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use qlrs_check::{Checker, Invariant};
use qlrs_ledger::Ledger;
use qlrs_schema::{AliveMask, Kind, Status};
use qlrs_testkit::{Defect, GeneratorConfig, generate, generate_rich};

fn prepared(bytes: Vec<u8>) -> qlrs_io::ParsedTrace {
    let mut trace = qlrs_io::ParsedTrace::parse(bytes).unwrap();
    Ledger::prepare(&mut trace.store);
    trace
}

#[test]
fn req_check_014_violation_dominates_uncovered_tail_and_is_reducible() {
    let fixture = generate(2, 42, Some((Defect::D03, 0))).unwrap();
    let bytes: Vec<_> = fixture
        .bytes
        .split_inclusive(|byte| *byte == b'\n')
        .take(9)
        .flatten()
        .copied()
        .collect();
    let trace = prepared(bytes);
    let report = Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        None,
        false,
    );
    assert_eq!(report.status, Status::Violation);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| { finding.reason.as_deref() == Some("missing-later-account-claim") })
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| { finding.reason.as_deref() == Some("missing-later-position-claim") })
    );
    let target = report.at_seq(fixture.target_seq.unwrap()).unwrap();
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    assert_eq!(reduced.target.instance_key, target.instance_key);
    assert!(reduced.retained_events < trace.store.len());
}

#[test]
fn req_reduce_009_i2_witness_needs_only_its_instrument_and_position_claims() {
    let fixture = generate_rich(
        GeneratorConfig::profile("sparse", 96, 42).unwrap(),
        Some((Defect::D06, 48)),
    )
    .unwrap();
    let bytes = String::from_utf8(fixture.bytes)
        .unwrap()
        .lines()
        .filter(|line| !line.contains("\"type\":\"AccountClaim\""))
        .fold(String::new(), |mut out, line| {
            out.push_str(line);
            out.push('\n');
            out
        })
        .into_bytes();
    let trace = prepared(bytes);
    assert_eq!(trace.store.cash_claims.raw, [] as [i128; 0]);
    let original = Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        None,
        false,
    );
    assert_eq!(original.status, Status::Violation);
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let key = reduced.target.instance_key.as_ref().unwrap();
    assert_eq!(key.invariant_id, Invariant::I2);
    let mut claims = 0;
    let mut fills = 0;
    for event in 0..trace.store.len() {
        if !reduced.alive.contains_event(&trace.store, event) {
            continue;
        }
        let kind = *trace.store.spine.kind.get(event).unwrap();
        let payload = *trace.store.spine.payload_idx.get(event).unwrap() as usize;
        assert_ne!(kind, Kind::CashClaim);
        if kind == Kind::PositionClaim {
            let instrument = *trace.store.position_claims.instrument.get(payload).unwrap();
            assert_eq!(
                trace
                    .store
                    .header
                    .instruments
                    .get(instrument as usize)
                    .unwrap()
                    .id,
                key.scope
            );
            claims += 1;
        }
        if kind == Kind::Filled {
            let order = *trace.store.fills.order.get(payload).unwrap() as usize;
            let instrument = *trace.store.orders.instrument.get(order).unwrap() as usize;
            assert_eq!(
                trace.store.header.instruments.get(instrument).unwrap().id,
                key.scope
            );
            fills += 1;
        }
    }
    assert_eq!(claims, 2);
    assert!(fills > 0);
    let dir = std::env::temp_dir().join(format!("qlrs-cp20-scope-{}", std::process::id()));
    qlrs_bundle::write(&dir, &trace, &reduced).unwrap();
    qlrs_bundle::verify(&dir).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn req_reduce_010_r1_d04_sizes_finish_under_existing_budgets() {
    for orders in [400, 2000] {
        let fixture = generate(orders, 42, Some((Defect::D04, orders / 2))).unwrap();
        let trace = prepared(fixture.bytes);
        let result = qlrs_reduce::reduce_with_limits(
            &trace.store,
            &qlrs_io::digest(&trace.source),
            None,
            fixture.target_seq,
            qlrs_reduce::ReductionLimits {
                max_calls: 200,
                time_limit: std::time::Duration::from_secs(5),
            },
        )
        .unwrap();
        assert_eq!(
            result.target.instance_key.as_ref().unwrap().invariant_id,
            Invariant::I1
        );
        assert!(result.closure_1_minimal);
        assert!(result.predicate_calls <= 200);
    }
}
