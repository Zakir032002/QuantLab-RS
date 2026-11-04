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

#[test]
fn req_gate_002_shapes_have_real_removals_and_recheckable_certificates() {
    use qlrs_testkit::{DEFECTS, SHAPES, generate_shape};
    let mut tested = 0;
    for shape in SHAPES {
        for defect in DEFECTS.into_iter().take(11) {
            let fixture = generate_shape(shape, 3, 1000, Some(defect)).unwrap();
            let trace = prepared(fixture.bytes);
            let result = qlrs_reduce::reduce(
                &trace.store,
                &qlrs_io::digest(&trace.source),
                None,
                fixture.target_seq,
            )
            .unwrap_or_else(|error| panic!("{shape} {defect:?}: {error}"));
            assert!(result.closure_1_minimal, "{shape} {defect:?}");
            assert!(result.certificate.complete);
            assert_eq!(
                result.certificate.vacuous,
                result.certificate.candidates_tested == 0
            );
            tested += result.certificate.candidates_tested;
            let noise = trace.store.order_index("NOISE").unwrap();
            assert!(
                !result
                    .alive
                    .contains(*trace.store.orders.unit.get(noise).unwrap() as usize)
            );
            let dir = std::env::temp_dir().join(format!(
                "qlrs-p2-cert-{}-{shape}-{defect:?}",
                std::process::id()
            ));
            qlrs_bundle::write(&dir, &trace, &result).unwrap();
            qlrs_bundle::verify_with_options(&dir, true)
                .unwrap_or_else(|error| panic!("{shape} {defect:?}: {error}"));
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    assert!(tested > 0);
}

#[test]
fn req_reduce_012_long_lived_and_second_carried_error_are_bounded() {
    use qlrs_testkit::generate_shape;
    for shape in ["S3", "S6"] {
        let fixture = generate_shape(shape, 42, 12_000, Some(Defect::D04)).unwrap();
        let trace = prepared(fixture.bytes);
        let original = Checker::check(
            &trace.store,
            &AliveMask::all(trace.store.units.len()),
            None,
            true,
        );
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
        assert!(result.closure_1_minimal);
        assert!(result.predicate_calls <= 200);
        assert_eq!(
            result.target.instance_key,
            original
                .at_seq(fixture.target_seq.unwrap())
                .unwrap()
                .instance_key
        );
        if shape == "S6" {
            assert!(
                original
                    .keys()
                    .any(|key| key.anchor_claim_original_seq < fixture.target_seq.unwrap())
            );
            assert!(result.target.window.start_seq.is_some());
        }
    }
    let fixture = generate_rich(
        GeneratorConfig::profile("long-lived", 2400, 42).unwrap(),
        Some((Defect::D07, 0)),
    )
    .unwrap();
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
    assert!(result.closure_1_minimal);
    assert!(result.checkpoint.as_ref().unwrap().cut_seq > 10_000);
    assert!(result.predicate_calls <= 200);
}

#[test]
fn req_bundle_002_v1_roles_and_certificates_reject_false_proofs() {
    let fixture = qlrs_testkit::generate_shape("S9", 3, 1000, Some(Defect::D06)).unwrap();
    let trace = prepared(fixture.bytes);
    let result = qlrs_reduce::reduce(
        &trace.store,
        &qlrs_io::digest(&trace.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("qlrs-proof-tamper-{}", std::process::id()));
    let manifest = qlrs_bundle::write(&dir, &trace, &result).unwrap();
    assert_eq!(manifest.schema, "qlrs-bundle/v1");
    assert_eq!(
        manifest.certificate_candidates_tested,
        Some(result.certificate.candidates_tested)
    );
    let roles: Vec<qlrs_bundle::RoleEntry> =
        qlrs_io::decode(&std::fs::read(dir.join("witness_roles.json")).unwrap()).unwrap();
    assert_eq!(
        roles
            .iter()
            .filter(|entry| entry.role == qlrs_bundle::WitnessRole::Anchor)
            .count(),
        1
    );
    assert_eq!(
        roles
            .iter()
            .filter(|entry| entry.role == qlrs_bundle::WitnessRole::Asserted)
            .count(),
        1
    );
    assert!(
        roles
            .iter()
            .any(|entry| entry.role == qlrs_bundle::WitnessRole::Synthetic)
    );
    assert!(
        roles
            .iter()
            .any(|entry| entry.role == qlrs_bundle::WitnessRole::Evidence)
    );
    let mut witness = qlrs_io::ParsedTrace::read(dir.join("reduced_trace.jsonl")).unwrap();
    let checkpoint: qlrs_schema::SyntheticCheckpoint =
        qlrs_io::decode(&std::fs::read(dir.join("synthetic_checkpoint.json")).unwrap()).unwrap();
    assert_eq!(checkpoint.schema_version, 1);
    qlrs_ledger::Ledger::prepare_with_checkpoint(&mut witness.store, Some(&checkpoint)).unwrap();
    let check = |certificate: &qlrs_reduce::Certificate| {
        qlrs_reduce::recheck_certificate(&witness.store, Some(&checkpoint), certificate)
    };
    check(&result.certificate).unwrap();
    let mut bad = result.certificate.clone();
    bad.candidates_tested += 1;
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.graph_digest = "0".repeat(64);
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.entries.pop();
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.original_keys.clear();
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.pinned.clear();
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.kept_units.pop();
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.entries.first_mut().unwrap().removed_closure = vec![[0, u32::MAX]];
    assert!(check(&bad).is_err());
    let mut bad = result.certificate.clone();
    bad.entries.first_mut().unwrap().outcome = qlrs_reduce::Rejection::ResidualChanged {
        new_residual: "123456789".into(),
    };
    assert!(check(&bad).is_err());
    let log = std::fs::read(dir.join("reduction_log.jsonl")).unwrap();
    assert!(
        std::str::from_utf8(&log)
            .unwrap()
            .contains("qlrs-reduce-log/v1")
    );
    assert!(log.len() < 16_384);
    qlrs_bundle::verify_with_options(&dir, true).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn req_reduce_016_unit_renaming_preserves_observed_witness() {
    let fixture = qlrs_testkit::generate_shape("S5", 42, 1000, Some(Defect::D06)).unwrap();
    let first = prepared(fixture.bytes.clone());
    let mut second = prepared(fixture.bytes);
    let count = second.store.units.len();
    second.store.units.reverse();
    for unit in &mut second.store.spine.unit {
        *unit = u32::try_from(count.saturating_sub(1).saturating_sub(*unit as usize)).unwrap();
    }
    for unit in &mut second.store.orders.unit {
        *unit = u32::try_from(count.saturating_sub(1).saturating_sub(*unit as usize)).unwrap();
    }
    let a = qlrs_reduce::reduce(
        &first.store,
        &qlrs_io::digest(&first.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let b = qlrs_reduce::reduce(
        &second.store,
        &qlrs_io::digest(&second.source),
        None,
        fixture.target_seq,
    )
    .unwrap();
    let mut left = Vec::new();
    first.write_kept(&a.alive, &mut left).unwrap();
    let mut right = Vec::new();
    second.write_kept(&b.alive, &mut right).unwrap();
    assert_eq!(left, right);
    assert_eq!(a.target.instance_key, b.target.instance_key);
    assert_eq!(a.checkpoint, b.checkpoint);
}

#[test]
fn req_reduce_017_nonmonotone_superset_is_saved_and_seed_recovers() {
    use qlrs_reduce::{Predicate, ReductionPredicate};
    let trace =
        prepared(include_bytes!("../../../fixtures/part2/nonmonotone-extra-order.jsonl").to_vec());
    let original = Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        None,
        true,
    );
    let key = original.at_seq(7).unwrap().instance_key.clone().unwrap();
    let mut predicate = ReductionPredicate::new(
        &trace.store,
        key.clone(),
        &original,
        None,
        &qlrs_io::digest(&trace.source),
    )
    .unwrap();
    let b = predicate.candidate_mask(&AliveMask::all(trace.store.units.len()));
    let mut a = b.clone();
    let noise = trace.store.order_index("NOISE").unwrap();
    a.set(*trace.store.orders.unit.get(noise).unwrap() as usize, false);
    assert!(a.iter().all(|unit| b.contains(unit)));
    assert!(predicate.test(&a).unwrap().accepted);
    assert!(!predicate.test(&b).unwrap().accepted);
    let result =
        qlrs_reduce::reduce(&trace.store, &qlrs_io::digest(&trace.source), None, Some(7)).unwrap();
    assert!(result.closure_1_minimal);
    assert_eq!(result.target.instance_key, Some(key));
    assert!(
        !result
            .alive
            .contains(*trace.store.orders.unit.get(noise).unwrap() as usize)
    );
    println!("saved nonmonotone case: P(A)=true, P(B)=false, A subset B; SBR seed recovers");
}

#[test]
fn req_bundle_003_original_v0_writer_bundle_still_verifies() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/compat/bundle-v0");
    let manifest = qlrs_bundle::verify(&dir).unwrap();
    assert_eq!(manifest.schema, "qlrs-bundle/v0");
    assert!(qlrs_bundle::verify_with_options(&dir, true).is_err());
    let checkpoint: qlrs_schema::SyntheticCheckpoint =
        qlrs_io::decode(&std::fs::read(dir.join("synthetic_checkpoint.json")).unwrap()).unwrap();
    assert_eq!(checkpoint.schema_version, 0);
    assert!(
        !qlrs_io::encode(&checkpoint)
            .unwrap()
            .windows(b"schema_version".len())
            .any(|bytes| bytes == b"schema_version")
    );
}
