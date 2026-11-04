//! User-requested claim boundaries, lifecycle context, budgets and rich traces.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]
use qlrs_check::{Checker, Invariant};
use qlrs_ledger::Ledger;
use qlrs_schema::{AliveMask, Kind, OrderState, Status, SyntheticCheckpoint};
use qlrs_testkit::{DEFECTS, Defect, GeneratorConfig, generate, generate_rich};
use std::time::Duration;
fn parse(bytes: Vec<u8>, cp: Option<&SyntheticCheckpoint>) -> qlrs_io::ParsedTrace {
    let mut trace = qlrs_io::ParsedTrace::parse(bytes).unwrap();
    Ledger::prepare_with_checkpoint(&mut trace.store, cp).unwrap();
    trace
}
fn report(trace: &qlrs_io::ParsedTrace, cp: Option<&SyntheticCheckpoint>) -> qlrs_check::Report {
    Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        cp,
        false,
    )
}
fn temp() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "qlrs-corrections-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}
#[test]
fn req_check_011_every_fill_needs_later_cash_and_same_asset_position() {
    let bytes = generate(2, 0, None).unwrap().bytes;
    let text = String::from_utf8(bytes).unwrap();
    for (cash, position) in [(false, false), (true, false), (false, true)] {
        let mut selected = String::new();
        for line in text.lines() {
            if (!cash && line.contains("\"seq\":9,")) || (!position && line.contains("\"seq\":10,"))
            {
                continue;
            }
            selected.push_str(line);
            selected.push('\n');
        }
        let trace = parse(selected.into_bytes(), None);
        let result = report(&trace, None);
        assert_eq!(result.status, Status::Inconclusive);
        assert_eq!(
            result
                .findings
                .iter()
                .any(|finding| finding.reason.as_deref() == Some("missing-later-account-claim")),
            !cash
        );
        assert_eq!(
            result
                .findings
                .iter()
                .any(|finding| finding.reason.as_deref() == Some("missing-later-position-claim")),
            !position
        );
        assert!(
            result
                .findings
                .iter()
                .filter(|finding| finding
                    .reason
                    .as_deref()
                    .is_some_and(|r| r.starts_with("missing-later-")))
                .all(|finding| finding.window.end_seq == 8)
        );
    }
    let mut mixed = generate(2, 0, Some((Defect::D03, 0))).unwrap().bytes;
    let cutoff = mixed
        .split_inclusive(|b| *b == b'\n')
        .take(9)
        .map(<[u8]>::len)
        .sum();
    mixed.truncate(cutoff);
    let trace = parse(mixed, None);
    let result = report(&trace, None);
    assert_eq!(result.status, Status::Inconclusive);
    assert!(result.keys().any(|key| key.invariant_id == Invariant::I1));
}
#[test]
fn req_check_012_i2_uses_local_claim_windows_for_carried_errors() {
    let fixture = generate(4, 0, Some((Defect::D06, 1))).unwrap();
    let anchor = fixture.target_seq.unwrap();
    let text = String::from_utf8(fixture.bytes)
        .unwrap()
        .replace("\"net_qty\":\"50.000\"", "\"net_qty\":\"40.000\"")
        .replace("\"net_qty\":\"70.000\"", "\"net_qty\":\"60.000\"");
    let trace = parse(text.into_bytes(), None);
    let result = report(&trace, None);
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|finding| {
            finding
                .instance_key
                .as_ref()
                .is_some_and(|key| key.invariant_id == Invariant::I2)
        })
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings.first().unwrap().window.start_seq, Some(5));
    assert_eq!(findings.first().unwrap().window.end_seq, anchor);
    assert_eq!(
        findings.first().unwrap().residual.as_deref(),
        Some("-10.000")
    );
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        Some(anchor),
    )
    .unwrap();
    let fill = trace.store.spine.seq.binary_search(&8).unwrap();
    assert!(reduced.alive.contains_event(&trace.store, fill));
    let predecessor = trace.store.spine.seq.binary_search(&5).unwrap();
    assert!(reduced.alive.contains_event(&trace.store, predecessor));
    let mut removed = reduced.alive.clone();
    removed.set(*trace.store.spine.unit.get(fill).unwrap() as usize, false);
    let mut predicate = qlrs_reduce::ReductionPredicate::new(
        &trace.store,
        reduced.target.instance_key.clone().unwrap(),
        &result,
        None,
        &qlrs_io::digest(&trace.source),
    )
    .unwrap();
    assert!(
        !qlrs_reduce::Predicate::test(&mut predicate, &removed)
            .unwrap()
            .accepted
    );
    assert_eq!(
        Ledger::replay(&trace.store, &AliveMask::all(trace.store.units.len()), None)
            .unwrap()
            .positions
            .as_slice(),
        &[70_000]
    );
}
#[test]
fn req_checkpoint_001_earlier_bad_claim_is_an_opening_boundary() {
    let text = String::from_utf8(generate(4, 0, None).unwrap().bytes).unwrap();
    let mut out = String::new();
    let mut claim = 0;
    let mut earlier = String::new();
    for line in text.lines() {
        let mut line = line.to_owned();
        if let Some(start) = line.find("\"balance_total\":\"") {
            let start = start + "\"balance_total\":\"".len();
            let end = start + line.get(start..).unwrap().find('"').unwrap();
            let value = line
                .get(start..end)
                .unwrap()
                .parse::<qlrs_num::Fixed>()
                .unwrap();
            let bias = if claim >= 2 {
                200
            } else if claim >= 1 {
                100
            } else {
                0
            };
            let value = qlrs_num::Fixed::new(value.raw() - bias, value.scale())
                .unwrap()
                .to_string();
            line.replace_range(start..end, &value);
            if claim == 1 {
                earlier = value;
            }
            claim += 1;
        }
        out.push_str(&line);
        out.push('\n');
    }
    let trace = parse(out.into_bytes(), None);
    let original = report(&trace, None);
    assert_eq!(
        original
            .keys()
            .filter(|key| key.invariant_id == Invariant::I1)
            .count(),
        2
    );
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        Some(14),
    )
    .unwrap();
    let cp = reduced.checkpoint.as_ref().unwrap();
    assert_eq!(cp.cut_seq, 9);
    assert_eq!(cp.cash_window.as_ref().unwrap().observed, earlier);
    assert_ne!(cp.cash, earlier);
    let mut expected = Ledger::initial(&trace.store, None).unwrap();
    for event in 0..trace.store.len() {
        if *trace.store.spine.seq.get(event).unwrap() >= cp.cut_seq {
            break;
        }
        expected.apply(&trace.store, event).unwrap();
    }
    assert_eq!(
        cp.cash.parse::<qlrs_num::Fixed>().unwrap().raw(),
        expected.cash
    );
    let witness = Checker::check(&trace.store, &reduced.alive, Some(cp), false);
    assert_eq!(witness.keys().count(), 1);
    assert!(witness.at_seq(9).is_none());
    assert_eq!(
        witness.at_seq(14).unwrap().residual.as_deref(),
        Some("-1.00")
    );
    assert!(reduced.alive.contains_event(
        &trace.store,
        trace.store.spine.seq.binary_search(&9).unwrap()
    ));
}
#[test]
fn req_checkpoint_002_open_and_terminal_orders_and_prior_trades_survive_cut() {
    for (defect, state) in [
        (Defect::D07, OrderState::Canceled),
        (Defect::D08, OrderState::Partial),
        (Defect::D11, OrderState::Partial),
    ] {
        let fixture = generate_rich(
            GeneratorConfig::profile("long-lived", 96, 42).unwrap(),
            Some((defect, 0)),
        )
        .unwrap();
        let trace = parse(fixture.bytes, None);
        let reduced = qlrs_reduce::reduce(
            &trace.store,
            &qlrs_io::digest(&trace.source),
            None,
            fixture.target_seq,
        )
        .unwrap();
        let cp = reduced.checkpoint.as_ref().unwrap();
        let order = cp
            .orders
            .iter()
            .find(|order| order.client_order_id == "O0")
            .unwrap();
        assert_eq!(order.status, state);
        assert_eq!(order.filled_quantity, "5.000");
        let mut bytes = Vec::new();
        trace.write_kept(&reduced.alive, &mut bytes).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            !text
                .lines()
                .any(|line| line.contains("\"client_order_id\":\"O0\"")
                    && line.contains("OrderSubmitted"))
        );
        let witness = parse(bytes, Some(cp));
        let bounded = Ledger::replay(&trace.store, &reduced.alive, Some(cp)).unwrap();
        let written = Ledger::replay(
            &witness.store,
            &AliveMask::all(witness.store.units.len()),
            Some(cp),
        )
        .unwrap();
        assert_eq!(bounded.cash, written.cash);
        assert_eq!(bounded.positions, written.positions);
        let all = AliveMask::all(trace.store.units.len());
        let full = Ledger::replay(&trace.store, &all, None).unwrap();
        let resumed = Ledger::replay(&trace.store, &all, Some(cp)).unwrap();
        assert_eq!(resumed.cash, full.cash);
        assert_eq!(resumed.positions, full.positions);
        assert!(
            report(&witness, Some(cp))
                .keys()
                .any(|key| Some(key) == reduced.target.instance_key.as_ref())
        );
        if defect == Defect::D11 {
            assert!(cp.trades.iter().any(|trade| trade.trade_id == "T0-0"));
            let mut broken = cp.clone();
            broken.trades.clear();
            assert!(
                !report(&witness, Some(&broken))
                    .keys()
                    .any(|key| Some(key) == reduced.target.instance_key.as_ref())
            );
        } else {
            let mut broken = cp.clone();
            let order = broken
                .orders
                .iter_mut()
                .find(|order| order.client_order_id == "O0")
                .unwrap();
            if defect == Defect::D07 {
                order.status = OrderState::Accepted;
            } else {
                order.filled_quantity = "0.000".into();
            }
            assert!(
                !report(&witness, Some(&broken))
                    .keys()
                    .any(|key| Some(key) == reduced.target.instance_key.as_ref())
            );
        }
        let again = qlrs_reduce::reduce(
            &witness.store,
            &qlrs_io::digest(&witness.source),
            Some(cp),
            fixture.target_seq,
        )
        .unwrap();
        let mut twice = Vec::new();
        witness.write_kept(&again.alive, &mut twice).unwrap();
        assert_eq!(twice, witness.source);
        assert_eq!(again.checkpoint.as_ref(), Some(cp));
    }
    let fixture = generate_rich(
        GeneratorConfig::profile("long-lived", 96, 42).unwrap(),
        Some((Defect::D08, 0)),
    )
    .unwrap();
    let reference = parse(fixture.bytes.clone(), None);
    let alias = reference
        .store
        .strings
        .get(*reference.store.fills.trade.first().unwrap())
        .unwrap();
    let renamed = String::from_utf8(fixture.bytes).unwrap().replace(
        "\"client_order_id\":\"O0\"",
        &format!("\"client_order_id\":\"{alias}\""),
    );
    let trace = parse(renamed.into_bytes(), None);
    assert!(
        trace
            .store
            .orders
            .id
            .windows(2)
            .any(|pair| pair.first() > pair.get(1))
    );
    assert!(trace.store.order_index(alias).is_some());
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let cp = reduced.checkpoint.as_ref().unwrap();
    let order = cp
        .orders
        .iter()
        .find(|order| order.client_order_id == alias)
        .unwrap();
    assert_eq!(order.status, OrderState::Partial);
    assert_eq!(order.filled_quantity, "5.000");
    let mut bytes = Vec::new();
    trace.write_kept(&reduced.alive, &mut bytes).unwrap();
    let witness = parse(bytes, Some(cp));
    assert!(
        report(&witness, Some(cp))
            .keys()
            .any(|key| Some(key) == reduced.target.instance_key.as_ref())
    );
}
#[test]
fn req_graph_002_checkpoint_inventory_replaces_historical_buy_dependencies() {
    let fixture = generate_rich(
        GeneratorConfig::profile("interleaved", 48, 42).unwrap(),
        Some((Defect::D04, 24)),
    )
    .unwrap();
    let trace = parse(fixture.bytes, None);
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    assert!(reduced.checkpoint.is_some());
    let graph = qlrs_reduce::UnitGraph::build_with_checkpoint(
        &trace.store,
        reduced.checkpoint.as_ref(),
        &reduced.alive,
    )
    .unwrap();
    assert_eq!(graph.parents, [] as [u32; 0]);
    assert_ne!(
        qlrs_reduce::UnitGraph::build(&trace.store).unwrap().parents,
        [] as [u32; 0]
    );
    let mut missing = reduced.checkpoint.clone().unwrap();
    for position in &mut missing.positions {
        position.net_qty = "0.000".into();
    }
    let target_order = trace.store.order_index("O24").unwrap();
    let mut sell_only = AliveMask::empty(trace.store.units.len())
        .with_range(reduced.alive.bounds().0, reduced.alive.bounds().1);
    sell_only.set(
        *trace.store.orders.unit.get(target_order).unwrap() as usize,
        true,
    );
    assert!(
        qlrs_reduce::UnitGraph::build_with_checkpoint(&trace.store, Some(&missing), &sell_only)
            .is_err()
    );
}
#[test]
fn req_reduce_005_limits_return_best_verified_uncertified_bundles() {
    let fixture = generate_rich(
        GeneratorConfig::profile("sparse", 64, 42).unwrap(),
        Some((Defect::D03, 32)),
    )
    .unwrap();
    let trace = parse(fixture.bytes, None);
    for (calls, time, reason) in [
        (
            0,
            Duration::from_secs(30),
            qlrs_reduce::LimitReason::CallLimit,
        ),
        (
            1,
            Duration::from_secs(30),
            qlrs_reduce::LimitReason::CallLimit,
        ),
        (
            2,
            Duration::from_secs(30),
            qlrs_reduce::LimitReason::CallLimit,
        ),
        (100, Duration::ZERO, qlrs_reduce::LimitReason::TimeLimit),
    ] {
        let reduced = qlrs_reduce::reduce_with_limits(
            &trace.store,
            &qlrs_io::digest(&trace.source),
            None,
            fixture.target_seq,
            qlrs_reduce::ReductionLimits {
                max_calls: calls,
                time_limit: time,
            },
        )
        .unwrap();
        assert!(!reduced.closure_1_minimal);
        assert_eq!(reduced.stop_reason, Some(reason));
        assert!(reduced.predicate_calls <= calls);
        for entry in &reduced.log {
            let bytes = qlrs_io::encode(entry).unwrap();
            assert!(bytes.len() < 256);
            assert!(
                !std::str::from_utf8(&bytes)
                    .unwrap()
                    .contains("\"units_removed\":[")
            );
        }
        let path = temp();
        let manifest = qlrs_bundle::write(&path, &trace, &reduced).unwrap();
        assert!(!manifest.closure_1_minimal);
        assert_eq!(manifest.stop_reason, Some(reason));
        assert!(manifest.minimality_guarantee.contains("not certified"));
        assert_eq!(qlrs_bundle::verify(&path).unwrap(), manifest);
        std::fs::remove_dir_all(path).unwrap();
    }
}
#[test]
fn req_gate_001_all_eleven_defects_on_rich_profiles() {
    for profile in ["interleaved", "long-lived", "sparse"] {
        let config = GeneratorConfig::profile(profile, 64, 42).unwrap();
        let clean = parse(generate_rich(config.clone(), None).unwrap().bytes, None);
        assert_eq!(report(&clean, None).status, Status::Consistent, "{profile}");
        for defect in DEFECTS.into_iter().take(11) {
            let index = if profile == "long-lived" { 0 } else { 32 };
            let fixture = generate_rich(config.clone(), Some((defect, index))).unwrap();
            let anchor = fixture.target_seq.unwrap();
            let trace = parse(fixture.bytes, None);
            let original = report(&trace, None);
            assert_eq!(
                original.status,
                Status::Violation,
                "{profile} {defect:?}: {original:#?}"
            );
            let expected = if defect == Defect::D06 {
                Invariant::I2
            } else if matches!(
                defect,
                Defect::D07 | Defect::D08 | Defect::D09 | Defect::D11
            ) {
                Invariant::I3
            } else {
                Invariant::I1
            };
            assert_eq!(
                original
                    .at_seq(anchor)
                    .unwrap()
                    .instance_key
                    .as_ref()
                    .unwrap()
                    .invariant_id,
                expected
            );
            let reduced = qlrs_reduce::reduce_with_limits(
                &trace.store,
                &qlrs_io::digest(&trace.source),
                None,
                Some(anchor),
                qlrs_reduce::ReductionLimits {
                    max_calls: 1024,
                    time_limit: Duration::from_secs(2),
                },
            )
            .unwrap();
            assert!(reduced.closure_1_minimal, "{profile} {defect:?}");
            assert!(
                reduced.retained_events <= 128,
                "{profile} {defect:?}: {}",
                reduced.retained_events
            );
            let mut bytes = Vec::new();
            trace.write_kept(&reduced.alive, &mut bytes).unwrap();
            for line in bytes.split_inclusive(|b| *b == b'\n') {
                assert!(
                    trace
                        .source
                        .split_inclusive(|b| *b == b'\n')
                        .any(|original| original == line)
                );
            }
            let witness = parse(bytes, reduced.checkpoint.as_ref());
            let result = report(&witness, reduced.checkpoint.as_ref());
            assert_eq!(result.status, Status::Violation);
            assert!(
                result
                    .keys()
                    .any(|key| Some(key) == reduced.target.instance_key.as_ref())
            );
        }
    }
}
#[test]
fn req_generator_001_interleaving_sparse_claims_long_lived_and_carried_offsets() {
    let config = GeneratorConfig::profile("long-lived", 96, 123).unwrap();
    let fixture = generate_rich(config.clone(), Some((Defect::D01, 0))).unwrap();
    let trace = parse(fixture.bytes, None);
    assert!(trace.store.header.instruments.len() > 1);
    let order = trace.store.order_index("O0").unwrap();
    let unit = *trace.store.orders.unit.get(order).unwrap();
    let events: Vec<_> = trace
        .store
        .spine
        .unit
        .iter()
        .enumerate()
        .filter(|(_, value)| **value == unit)
        .map(|(event, _)| event)
        .collect();
    assert!(events.last().unwrap() - events.first().unwrap() > trace.store.len() / 2);
    let other = trace.store.order_index("O10").unwrap();
    let next = trace.store.order_index("O11").unwrap();
    let other_unit = *trace.store.orders.unit.get(other).unwrap();
    let next_unit = *trace.store.orders.unit.get(next).unwrap();
    let other_start = trace
        .store
        .spine
        .unit
        .iter()
        .position(|unit| *unit == other_unit)
        .unwrap();
    let next_start = trace
        .store
        .spine
        .unit
        .iter()
        .position(|unit| *unit == next_unit)
        .unwrap();
    let other_end = trace
        .store
        .spine
        .unit
        .iter()
        .rposition(|unit| *unit == other_unit)
        .unwrap();
    assert!(other_start < next_start && next_start < other_end);
    let mut max_fills = 0;
    let mut fills = 0;
    for kind in &trace.store.spine.kind {
        if *kind == Kind::Filled {
            fills += 1;
            max_fills = max_fills.max(fills);
        } else if *kind == Kind::CashClaim {
            fills = 0;
        }
    }
    assert!(max_fills >= 8);
    assert_eq!(
        report(&trace, None)
            .keys()
            .filter(|key| key.invariant_id == Invariant::I1)
            .count(),
        1
    );
    let carried = parse(
        generate_rich(config.clone(), Some((Defect::D03, 48)))
            .unwrap()
            .bytes,
        None,
    );
    assert_eq!(
        report(&carried, None)
            .keys()
            .filter(|key| key.invariant_id == Invariant::I1)
            .count(),
        1
    );
    let mut corrected = config;
    corrected.carry_errors = false;
    let fixture = generate_rich(corrected, Some((Defect::D03, 48))).unwrap();
    let trace = parse(fixture.bytes, None);
    assert_eq!(
        report(&trace, None)
            .keys()
            .filter(|key| key.invariant_id == Invariant::I1)
            .count(),
        2
    );
}

#[test]
fn req_reduce_006_sparse_dense_and_incremental_patterns_match_on_all_defects() {
    use qlrs_reduce::Predicate;
    for defect in DEFECTS.into_iter().take(11) {
        let fixture = generate_rich(
            GeneratorConfig::profile("sparse", 64, 42).unwrap(),
            Some((defect, 32)),
        )
        .unwrap();
        let trace = parse(fixture.bytes, None);
        let original = report(&trace, None);
        let key = original
            .at_seq(fixture.target_seq.unwrap())
            .unwrap()
            .instance_key
            .clone()
            .unwrap();
        let mut predicate = qlrs_reduce::ReductionPredicate::new(
            &trace.store,
            key.clone(),
            &original,
            None,
            &qlrs_io::digest(&trace.source),
        )
        .unwrap();
        let mut random = 41_u64;
        for round in 0..40 {
            let mut mask = predicate.candidate_mask(&AliveMask::all(trace.store.units.len()));
            for unit in mask.iter().collect::<Vec<_>>() {
                random = random
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                if round > 0 && random.is_multiple_of(7) {
                    mask.set(unit, false);
                }
            }
            let cp = predicate.candidate_checkpoint(&mask).unwrap();
            let dense = Checker::check(&trace.store, &mask, cp.as_ref(), true);
            let selected: Vec<_> = (0..trace.store.len())
                .filter(|event| mask.contains_event(&trace.store, *event))
                .collect();
            assert_eq!(
                dense,
                Checker::check_selected(&trace.store, &mask, cp.as_ref(), true, &selected)
            );
            let admitted = dense.status == Status::Violation
                && dense.keys().any(|candidate| *candidate == key)
                && dense.findings.iter().all(|finding| {
                    !matches!(finding.status, Status::Inconclusive | Status::Unsupported)
                })
                && dense
                    .keys()
                    .all(|candidate| original.keys().any(|key| key == candidate))
                && dense
                    .keys()
                    .filter(|key| key.invariant_id == Invariant::I3)
                    .all(|candidate| *candidate == key);
            assert_eq!(
                predicate.test(&mask).unwrap().accepted,
                admitted,
                "{defect:?} round={round}"
            );
            for finding in dense
                .findings
                .iter()
                .filter(|finding| finding.status == Status::Violation)
            {
                let key = finding.instance_key.as_ref().unwrap();
                let anchor = trace
                    .store
                    .spine
                    .seq
                    .binary_search(&finding.window.end_seq)
                    .unwrap();
                let anchor_unit = *trace.store.spine.unit.get(anchor).unwrap();
                let mut counts = [0_u64; 8];
                for &event in &selected {
                    let seq = *trace.store.spine.seq.get(event).unwrap();
                    let inside = if key.invariant_id == Invariant::I3 {
                        trace.store.spine.unit.get(event) == Some(&anchor_unit)
                            && finding.window.start_seq.is_none_or(|first| seq >= first)
                    } else {
                        finding.window.start_seq.is_none_or(|first| seq > first)
                    };
                    if inside && seq <= finding.window.end_seq {
                        let index = *trace.store.spine.kind.get(event).unwrap() as usize;
                        *counts.get_mut(index).unwrap() += 1;
                    }
                }
                let pattern: Vec<_> = [
                    Kind::Submitted,
                    Kind::Accepted,
                    Kind::Rejected,
                    Kind::Filled,
                    Kind::Canceled,
                    Kind::CashClaim,
                    Kind::PositionClaim,
                    Kind::Unsupported,
                ]
                .into_iter()
                .zip(counts)
                .filter(|(_, count)| *count > 0)
                .collect();
                let raw = key.residual_raw.parse::<i128>().unwrap();
                let sign = if raw < 0 { -1 } else { i8::from(raw > 0) };
                assert_eq!(
                    finding.class_fp.as_deref(),
                    Some(
                        qlrs_check::class_fingerprint(
                            key.invariant_id,
                            &key.sub_code,
                            &pattern,
                            sign,
                            &finding.relation
                        )
                        .as_str()
                    )
                );
            }
        }
    }
}

proptest::proptest! {
 #![proptest_config(proptest::test_runner::Config {cases:100,..proptest::test_runner::Config::default()})]
 #[test]
 fn req_generator_002_rich_statuses_across_seeds(seed in proptest::prelude::any::<u64>(),index in 0usize..32,profile in 0usize..3) {
    let name=["interleaved","long-lived","sparse"].get(profile).unwrap();let config=GeneratorConfig::profile(name,32,seed).unwrap();let clean=parse(generate_rich(config.clone(),None).unwrap().bytes,None);proptest::prop_assert_eq!(report(&clean,None).status,Status::Consistent);
    for defect in DEFECTS.into_iter().take(11) {let fixture=generate_rich(config.clone(),Some((defect,index))).unwrap();let trace=parse(fixture.bytes,None);let result=report(&trace,None);proptest::prop_assert_eq!(result.status,Status::Violation,"{:?}",defect);proptest::prop_assert!(result.at_seq(fixture.target_seq.unwrap()).is_some());}
 }
}

#[test]
fn req_checkpoint_003_invalid_context_never_becomes_a_financial_accusation() {
    let fixture = generate_rich(
        GeneratorConfig::profile("long-lived", 48, 42).unwrap(),
        Some((Defect::D08, 0)),
    )
    .unwrap();
    let trace = parse(fixture.bytes, None);
    let reduced = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let cp = reduced.checkpoint.unwrap();
    let mut invalid = Vec::new();
    let mut value = cp.clone();
    value.orders.push(value.orders.first().unwrap().clone());
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().instrument_id = "UNKNOWN".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().filled_quantity = "-1.000".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().quantity = "0.000".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().quantity = "bad".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().status = OrderState::Missing;
    invalid.push(value);
    let mut value = cp.clone();
    let order = value.orders.first_mut().unwrap();
    order.side = if order.side == qlrs_schema::Side::Buy {
        qlrs_schema::Side::Sell
    } else {
        qlrs_schema::Side::Buy
    };
    invalid.push(value);
    let mut value = cp.clone();
    value.orders.first_mut().unwrap().quantity = "999.000".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.cash_window.as_mut().unwrap().seq = Some(value.cut_seq + 1);
    invalid.push(value);
    let mut value = cp.clone();
    value.cash_window.as_mut().unwrap().observed = "bad".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.cash_window.as_mut().unwrap().delta = "bad".into();
    invalid.push(value);
    let mut value = cp.clone();
    value
        .position_windows
        .push(value.position_windows.first().unwrap().clone());
    invalid.push(value);
    let mut value = cp.clone();
    value.position_windows.first_mut().unwrap().instrument_id = "UNKNOWN".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.position_windows.first_mut().unwrap().delta = "bad".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.position_windows.first_mut().unwrap().seq = Some(value.cut_seq + 1);
    invalid.push(value);
    let mut value = cp.clone();
    value.cash = "-1.00".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.positions.pop();
    invalid.push(value);
    let mut value = cp.clone();
    value
        .positions
        .push(value.positions.first().unwrap().clone());
    invalid.push(value);
    let mut value = cp.clone();
    value.positions.first_mut().unwrap().instrument_id = "UNKNOWN".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.provenance = "observed".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.source_digest = "not-a-digest".into();
    invalid.push(value);
    let mut value = cp.clone();
    value.source_digest = "x".repeat(64);
    invalid.push(value);
    let mut value = cp.clone();
    value.description = "invented".into();
    invalid.push(value);
    let mut value = cp;
    value.trades.push(qlrs_schema::CheckpointTrade {
        instrument_id: "UNKNOWN".into(),
        trade_id: "T0-0".into(),
    });
    invalid.push(value);
    for (index, cp) in invalid.iter().enumerate() {
        let mut fresh = qlrs_io::ParsedTrace::parse(trace.source.clone()).unwrap();
        if Ledger::prepare_with_checkpoint(&mut fresh.store, Some(cp)).is_err()
            || Ledger::initial(&fresh.store, Some(cp)).is_err()
        {
            continue;
        }
        assert_eq!(
            report(&fresh, Some(cp)).status,
            Status::Inconclusive,
            "case {index}"
        );
    }
}
#[test]
fn req_check_013_sparse_indices_must_be_unique_and_in_sequence_order() {
    let trace = parse(generate(2, 0, None).unwrap().bytes, None);
    let mask = AliveMask::all(trace.store.units.len());
    for events in [vec![0, 0], vec![2, 1], vec![trace.store.len()]] {
        let result = Checker::check_selected(&trace.store, &mask, None, false, &events);
        assert_eq!(result.status, Status::Inconclusive);
        assert_eq!(result.keys().count(), 0);
    }
}
#[test]
fn req_reduce_007_other_lifecycle_breaches_are_not_admitted_or_returned_on_limits() {
    let fixture = generate_rich(
        GeneratorConfig::profile("sparse", 64, 42).unwrap(),
        Some((Defect::D03, 32)),
    )
    .unwrap();
    let anchor = fixture.target_seq.unwrap();
    let trace = parse(fixture.bytes, None);
    let target = report(&trace, None).at_seq(anchor).unwrap().clone();
    let fill = (0..trace.store.len())
        .find(|event| {
            trace.store.spine.kind.get(*event) == Some(&Kind::Filled)
                && trace
                    .store
                    .spine
                    .seq
                    .get(*event)
                    .is_some_and(|seq| target.window.start_seq.is_none_or(|start| *seq > start))
        })
        .unwrap();
    let payload = *trace.store.spine.payload_idx.get(fill).unwrap() as usize;
    let old = trace
        .store
        .strings
        .get(*trace.store.fills.trade.get(payload).unwrap())
        .unwrap();
    let prefix = old.strip_suffix("-1").unwrap();
    let source = String::from_utf8(trace.source).unwrap().replacen(
        &format!("\"trade_id\":\"{old}\""),
        &format!("\"trade_id\":\"{prefix}-0\""),
        1,
    );
    let trace = parse(source.into_bytes(), None);
    let result = report(&trace, None);
    assert!(result.keys().any(|key| key.invariant_id == Invariant::I3));
    for calls in [0, 100] {
        assert!(
            qlrs_reduce::reduce_with_limits(
                &trace.store,
                &qlrs_io::digest(&trace.source),
                None,
                Some(anchor),
                qlrs_reduce::ReductionLimits {
                    max_calls: calls,
                    time_limit: Duration::from_secs(30)
                }
            )
            .is_err()
        );
    }
}

#[test]
fn req_generator_003_each_instrument_has_independent_buy_sell_cycles() {
    let fixture = generate_rich(
        GeneratorConfig::profile("interleaved", 4000, 42).unwrap(),
        Some((Defect::D04, 2000)),
    )
    .unwrap();
    let trace = parse(fixture.bytes, None);
    assert_eq!(report(&trace, None).status, Status::Violation);
    for instrument in 0..trace.store.header.instruments.len() {
        for side in [qlrs_schema::Side::Buy, qlrs_schema::Side::Sell] {
            assert!(
                trace
                    .store
                    .orders
                    .instrument
                    .iter()
                    .zip(&trace.store.orders.side)
                    .any(|(index, value)| *index as usize == instrument && *value == side)
            );
        }
    }
    assert!(
        Ledger::replay(&trace.store, &AliveMask::all(trace.store.units.len()), None)
            .unwrap()
            .positions
            .into_iter()
            .all(|quantity| quantity > 1_000_000)
    );
}

#[test]
fn req_reduce_008_short_logs_read_legacy_removed_unit_lists() {
    let bytes=b"{\"candidate_digest\":\"abcd\",\"units_removed\":[1,5,9],\"result\":true,\"cache_hit\":false,\"full_replay\":true}";
    let entry: qlrs_reduce::LogEntry = qlrs_io::decode(bytes).unwrap();
    assert_eq!(entry.units_removed, 3);
    let encoded = qlrs_io::encode(&entry).unwrap();
    assert!(
        std::str::from_utf8(&encoded)
            .unwrap()
            .contains("\"units_removed\":3")
    );
    assert_eq!(
        qlrs_io::decode::<qlrs_reduce::LogEntry>(&encoded).unwrap(),
        entry
    );
}
