//! Gate B-synthetic, exact predicates and deterministic offline evidence.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use proptest::prelude::*;
use qlrs_check::Checker;
use qlrs_ledger::Ledger;
use qlrs_reduce::{Predicate, ReductionPredicate, UnitGraph};
use qlrs_schema::AliveMask;
use qlrs_testkit::{Defect, generate};
fn prepared(bytes: Vec<u8>) -> qlrs_io::ParsedTrace {
    prepared_with_checkpoint(bytes, None)
}
fn prepared_with_checkpoint(
    bytes: Vec<u8>,
    checkpoint: Option<&qlrs_schema::SyntheticCheckpoint>,
) -> qlrs_io::ParsedTrace {
    let mut input = qlrs_io::ParsedTrace::parse(bytes).unwrap();
    Ledger::prepare_with_checkpoint(&mut input.store, checkpoint).unwrap();
    input
}
#[test]
fn req_reduce_001_three_defects_exact_replay_and_idempotence() {
    for defect in [Defect::D01, Defect::D03, Defect::D07] {
        let fixture = generate(100, 42, Some((defect, 60))).unwrap();
        let input = prepared(fixture.bytes);
        let digest = input.source_digest();
        let reduced = qlrs_reduce::reduce(&input.store, &digest, None, fixture.target_seq).unwrap();
        assert!(reduced.closure_1_minimal);
        assert!(
            reduced.retained_events <= 6,
            "{defect:?} {}",
            reduced.retained_events
        );
        let cp = reduced.checkpoint.as_ref().unwrap();
        assert_eq!(cp.provenance, "synthetic");
        assert_eq!(cp.source_digest, digest);
        let target = reduced.target.instance_key.as_ref().unwrap();
        let original = Checker::check(
            &input.store,
            &AliveMask::all(input.store.units.len()),
            None,
            true,
        );
        assert!(original.keys().any(|key| key == target));
        let mut bytes = Vec::new();
        input.write_kept(&reduced.alive, &mut bytes).unwrap();
        let output = prepared_with_checkpoint(bytes.clone(), Some(cp));
        let report = Checker::check(
            &output.store,
            &AliveMask::all(output.store.units.len()),
            Some(cp),
            false,
        );
        assert!(report.keys().any(|key| key == target));
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            assert!(
                input
                    .source
                    .split_inclusive(|b| *b == b'\n')
                    .any(|source| source == line)
            );
        }
        let again = qlrs_reduce::reduce(
            &output.store,
            &output.source_digest(),
            Some(cp),
            Some(target.anchor_claim_original_seq),
        )
        .unwrap();
        let mut twice = Vec::new();
        output.write_kept(&again.alive, &mut twice).unwrap();
        assert_eq!(bytes, twice);
        assert_eq!(again.checkpoint, reduced.checkpoint);
    }
}
#[test]
fn req_reduce_002_actual_closure_minimality_certificate() {
    let fixture = generate(15, 0, Some((Defect::D11, 10))).unwrap();
    let input = prepared(fixture.bytes);
    let digest = input.source_digest();
    let original = Checker::check(
        &input.store,
        &AliveMask::all(input.store.units.len()),
        None,
        true,
    );
    let reduced = qlrs_reduce::reduce(&input.store, &digest, None, fixture.target_seq).unwrap();
    let graph = UnitGraph::build(&input.store).unwrap();
    let pinned = qlrs_reduce::pinned_units(&input.store, &reduced.target).unwrap();
    let mut predicate = ReductionPredicate::new(
        &input.store,
        reduced.target.instance_key.clone().unwrap(),
        &original,
        None,
        &digest,
    )
    .unwrap();
    for unit in reduced.alive.iter().filter(|id| !pinned.contains(*id)) {
        let closure = graph.closure(&[unit]);
        if closure.iter().any(|id| pinned.contains(*id)) {
            continue;
        }
        let mut candidate = reduced.alive.clone();
        for remove in closure {
            candidate.set(remove, false);
        }
        assert!(!predicate.test(&candidate).unwrap().accepted);
    }
}
proptest! {
 #![proptest_config(ProptestConfig {cases:100,..ProptestConfig::default()})]
 #[test]
 fn req_reduce_003_property_preserves_instance_and_source(seed in any::<u64>(),index in 1usize..19){
  let fixture=generate(20,seed,Some((Defect::D03,index))).unwrap();let input=prepared(fixture.bytes);let reduced=qlrs_reduce::reduce(&input.store,&input.source_digest(),None,fixture.target_seq).unwrap();let report=Checker::check(&input.store,&reduced.alive,reduced.checkpoint.as_ref(),false);prop_assert!(report.keys().any(|key|Some(key)==reduced.target.instance_key.as_ref()));prop_assert!(reduced.retained_events<=6);
 }
}
#[test]
fn req_bundle_001_deterministic_roundtrip_and_every_file_tamper() {
    let fixture = generate(40, 2, Some((Defect::D03, 20))).unwrap();
    let input = prepared(fixture.bytes);
    let reduction = qlrs_reduce::reduce(
        &input.store,
        &input.source_digest(),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let temp = std::env::temp_dir().join(format!("qlrs-bundle-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let a = temp.join("a");
    let b = temp.join("b");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
    qlrs_bundle::write(&a, &input, &reduction).unwrap();
    qlrs_bundle::write(&b, &input, &reduction).unwrap();
    assert!(qlrs_bundle::verify(&a).is_ok());
    for entry in std::fs::read_dir(&a).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap();
        let original = std::fs::read(&path).unwrap();
        assert_eq!(original, std::fs::read(b.join(file)).unwrap());
        let mut changed = original.clone();
        changed.push(b' ');
        std::fs::write(&path, changed).unwrap();
        assert!(
            qlrs_bundle::verify(&a).is_err(),
            "tamper accepted: {file:?}"
        );
        std::fs::write(&path, original).unwrap();
    }
    std::fs::remove_dir_all(&temp).unwrap();
}
#[test]
fn req_reduce_004_sparse_fast_predicate_matches_full_replay() {
    use qlrs_schema::Status;
    let fixture = generate(40, 42, Some((Defect::D03, 25))).unwrap();
    let input = prepared(fixture.bytes);
    let digest = input.source_digest();
    let store = &input.store;
    let original = Checker::check(store, &AliveMask::all(store.units.len()), None, true);
    let key = original
        .at_seq(fixture.target_seq.unwrap())
        .unwrap()
        .instance_key
        .clone()
        .unwrap();
    let reduction = qlrs_reduce::reduce(store, &digest, None, fixture.target_seq).unwrap();
    let mut predicate =
        ReductionPredicate::new(store, key.clone(), &original, None, &digest).unwrap();
    let mut random = 42_u64;
    for round in 0..200 {
        let mut mask = if round % 2 == 0 {
            reduction.alive.clone()
        } else {
            AliveMask::empty(store.units.len())
        };
        for unit in 0..store.units.len() {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            if random.is_multiple_of(40) {
                mask.set(unit, !mask.contains(unit));
            }
        }
        let mask = predicate.candidate_mask(&mask);
        let checkpoint = predicate.candidate_checkpoint(&mask).unwrap();
        let report = Checker::check_witness(store, &mask, checkpoint.as_ref(), &key);
        let reference = report.keys().any(|existing| *existing == key)
            && report.findings.iter().all(|finding| {
                !matches!(finding.status, Status::Unsupported | Status::Inconclusive)
            })
            && report
                .keys()
                .all(|existing| original.keys().any(|key| key == existing));
        assert_eq!(
            predicate.test(&mask).unwrap().accepted,
            reference,
            "round {round}"
        );
    }
}
#[test]
fn req_graph_001_sell_closures_consume_earlier_buy_inventory() {
    let fixture = generate(4, 0, Some((Defect::D04, 2))).unwrap();
    let text = String::from_utf8(fixture.bytes).unwrap().replace(
        "\"client_order_id\":\"O3\",\"instrument_id\":\"X\",\"side\":\"BUY\"",
        "\"client_order_id\":\"O3\",\"instrument_id\":\"X\",\"side\":\"SELL\"",
    );
    let mut input = prepared(text.into_bytes());
    let graph = UnitGraph::build(&input.store).unwrap();
    let units = &input.store.orders.unit;
    let a = *units.first().unwrap() as usize;
    let b = *units.get(1).unwrap() as usize;
    let c = *units.get(2).unwrap() as usize;
    let d = *units.get(3).unwrap() as usize;
    assert!(graph.closure(&[a]).contains(&c));
    assert!(!graph.closure(&[a]).contains(&d));
    assert!(graph.closure(&[b]).contains(&d));
    assert!(!graph.closure(&[b]).contains(&c));

    // The first sell leaves part of A; the next sell needs that remainder
    // followed by B. Returning a residual lot must preserve FIFO dependencies.
    *input.store.fill_qty.get_mut(0).unwrap() = 30_000;
    *input.store.fill_qty.get_mut(1).unwrap() = 10_000;
    let graph = UnitGraph::build(&input.store).unwrap();
    assert!(graph.closure(&[a]).contains(&c));
    assert!(graph.closure(&[a]).contains(&d));
    assert!(graph.closure(&[b]).contains(&d));
    assert!(!graph.closure(&[b]).contains(&c));
}
