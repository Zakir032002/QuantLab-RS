//! Independent full-checker differential tests for the numerical prefilter.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use proptest::{
    collection::vec,
    prelude::*,
    test_runner::{Config, TestRunner},
};
use qlrs_check::Checker;
use qlrs_reduce::{ContributionTable, FastPredicate, ReductionPredicate, UnitGraph};
use qlrs_schema::{AliveMask, Status};
use qlrs_testkit::{Defect, SHAPES, generate_shape};

#[test]
fn req_reduce_013_algebra_matches_full_checker_on_closed_subsets_per_shape() {
    let cases = std::env::var("QLRS_DIFFERENTIAL_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(128);
    for shape in SHAPES {
        for defect in [Defect::D03, Defect::D06] {
            let fixture = generate_shape(shape, 3, 400, Some(defect)).unwrap();
            let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
            qlrs_ledger::Ledger::prepare(&mut trace.store);
            let store = &trace.store;
            let original = Checker::check(store, &AliveMask::all(store.units.len()), None, true);
            let finding = original.at_seq(fixture.target_seq.unwrap()).unwrap();
            let key = finding.instance_key.clone().unwrap();
            let predicate = ReductionPredicate::new(
                store,
                key.clone(),
                &original,
                None,
                &qlrs_io::digest(&trace.source),
            )
            .unwrap();
            let window = predicate.candidate_mask(&AliveMask::all(store.units.len()));
            let checkpoint = predicate.candidate_checkpoint(&window).unwrap();
            let graph =
                UnitGraph::build_with_checkpoint(store, checkpoint.as_ref(), &window).unwrap();
            let pins = qlrs_reduce::pinned_units(store, finding).unwrap();
            let table = ContributionTable::new(store, &window, &key).unwrap();
            let ids = table.units.clone();
            let mut runner = TestRunner::new(Config {
                cases,
                failure_persistence: None,
                ..Config::default()
            });
            runner
                .run(&vec(any::<bool>(), ids.len()), |bits| {
                    let mut mask = window.clone();
                    for (unit, remove) in ids.iter().zip(bits) {
                        if !remove {
                            continue;
                        }
                        let closure = graph.closure(&[*unit as usize]);
                        if closure.iter().any(|unit| pins.contains(*unit)) {
                            continue;
                        }
                        for unit in closure {
                            mask.set(unit, false);
                        }
                    }
                    let checkpoint = predicate.candidate_checkpoint(&mask).unwrap();
                    let report = Checker::check_witness(store, &mask, checkpoint.as_ref(), &key);
                    prop_assert!(report.findings.iter().all(|finding| !matches!(
                        finding.status,
                        Status::Unsupported | Status::Inconclusive
                    )));
                    let actual = report
                        .keys()
                        .find(|other| {
                            other.invariant_id == key.invariant_id
                                && other.anchor_claim_original_seq == key.anchor_claim_original_seq
                        })
                        .map_or(0, |key| key.residual_raw.parse::<i128>().unwrap());
                    prop_assert_eq!(table.residual(&mask), Some(actual));
                    let end = mask.count().min(ids.len());
                    let prefix_sum = table
                        .values
                        .get(..end)
                        .unwrap()
                        .iter()
                        .try_fold(0_i128, |sum, value| sum.checked_add(*value))
                        .unwrap();
                    let total = table
                        .values
                        .iter()
                        .try_fold(0_i128, |sum, value| sum.checked_add(*value))
                        .unwrap();
                    let expected = key
                        .residual_raw
                        .parse::<i128>()
                        .unwrap()
                        .checked_add(total)
                        .unwrap()
                        .checked_sub(prefix_sum)
                        .unwrap();
                    prop_assert_eq!(table.prefix_residual(end), Some(expected));
                    let closures: Vec<_> = ids
                        .iter()
                        .map(|id| graph.prerequisites(&[*id as usize]))
                        .collect();
                    let empty = AliveMask::empty(store.units.len())
                        .with_range(window.bounds().0, window.bounds().1);
                    let sums = table.closure_prefixes(&closures, &empty).unwrap();
                    let mut direct = empty;
                    for (index, closure) in closures.iter().enumerate() {
                        for id in closure {
                            direct.set(*id, true);
                        }
                        prop_assert_eq!(
                            sums.get(index.saturating_add(1)).copied(),
                            table.residual(&direct)
                        );
                    }
                    Ok(())
                })
                .unwrap_or_else(|error| panic!("{shape} {defect:?}: {error}"));
        }
    }
    println!("differential: {cases} cases per invariant per shape, 9 shapes, no mismatches");
}
