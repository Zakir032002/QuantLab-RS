//! Rebased Gate B-synthetic; raw output is one JSON object per gate cell.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
use qlrs_check::{Checker, Invariant};
use qlrs_reduce::{ContributionTable, FastPredicate, Predicate, ReductionPredicate, UnitGraph};
use qlrs_schema::{AliveMask, Kind, Status};
use qlrs_testkit::{DEFECTS, SHAPES, generate_shape};
use std::time::{Duration, Instant};
fn exhaustive(
    trace: &qlrs_io::ParsedTrace,
    original: &qlrs_check::Report,
    key: &qlrs_check::InstanceKey,
    digest: &str,
) -> Option<usize> {
    let store = &trace.store;
    let mut predicate =
        ReductionPredicate::new(store, key.clone(), original, None, digest).unwrap();
    let window = predicate.candidate_mask(&AliveMask::all(store.units.len()));
    let table = ContributionTable::new(store, &window, key).unwrap();
    if table.units.len() > 20 {
        return None;
    }
    let cp = predicate.candidate_checkpoint(&window).unwrap();
    let graph = UnitGraph::build_with_checkpoint(store, cp.as_ref(), &window).unwrap();
    let target = original
        .findings
        .iter()
        .find(|finding| finding.instance_key.as_ref() == Some(key))
        .unwrap();
    let pins = qlrs_reduce::pinned_units(store, target).unwrap();
    let mut best = usize::MAX;
    for bits in 0..(1_u64 << table.units.len()) {
        let mut mask =
            AliveMask::empty(store.units.len()).with_range(window.bounds().0, window.bounds().1);
        for (index, id) in table.units.iter().enumerate() {
            if bits & (1_u64 << index) != 0 {
                mask.set(*id as usize, true);
            }
        }
        if pins.iter().any(|id| !mask.contains(id))
            || graph
                .prerequisites(&mask.iter().collect::<Vec<_>>())
                .iter()
                .any(|id| !mask.contains(*id))
        {
            continue;
        }
        if table
            .residual(&mask)
            .is_some_and(|value| value.to_string() != key.residual_raw)
        {
            continue;
        }
        if predicate.test(&mask).unwrap().accepted {
            let kept = (0..store.len())
                .filter(|event| mask.contains_event(store, *event))
                .count();
            best = best.min(kept);
        }
    }
    (best != usize::MAX).then_some(best)
}
fn main() {
    let quick = std::env::args().any(|arg| arg == "--quick");
    let sizes: &[usize] = if quick {
        &[1000]
    } else {
        &[1000, 10_000, 100_000]
    };
    let seeds: &[u64] = if quick { &[3] } else { &[1, 2, 3] };
    let base = std::env::temp_dir().join(format!("qlrs-part2-gate-{}", std::process::id()));
    std::fs::create_dir(&base).unwrap();
    let pools: Vec<_> = [1, 2, 8]
        .into_iter()
        .map(|threads| (threads, rayon_pool(threads)))
        .collect();
    let mut total = 0;
    let mut failures = 0;
    let mut genuine = 0;
    for shape in SHAPES {
        for size in sizes {
            for seed in seeds {
                for defect in DEFECTS.into_iter().take(11) {
                    let fixture = generate_shape(shape, *seed, *size, Some(defect)).unwrap();
                    let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
                    qlrs_ledger::Ledger::prepare(&mut trace.store);
                    let store = &trace.store;
                    let original =
                        Checker::check(store, &AliveMask::all(store.units.len()), None, true);
                    assert_eq!(original.status, Status::Violation);
                    let finding = original.at_seq(fixture.target_seq.unwrap()).unwrap();
                    let digest = qlrs_io::digest(&trace.source);
                    let key = finding.instance_key.as_ref().unwrap();
                    let mut canonical: Option<Vec<(String, Vec<u8>)>> = None;
                    let mut kept = 0;
                    let mut calls = 0;
                    let mut fast = 0;
                    let mut certificate_tests = 0;
                    let mut ns8 = 0;
                    let mut case_pass = true;
                    let mut log_bytes = 0;
                    let minimum = exhaustive(&trace, &original, key, &digest);
                    let mut required = 0;
                    let mut context = 0;
                    for (threads, pool) in &pools {
                        let start = Instant::now();
                        let result = pool
                            .install(|| {
                                qlrs_reduce::reduce_with_limits(
                                    store,
                                    &digest,
                                    None,
                                    fixture.target_seq,
                                    qlrs_reduce::ReductionLimits {
                                        max_calls: 10_000,
                                        time_limit: Duration::from_secs(30),
                                    },
                                )
                            })
                            .unwrap();
                        let elapsed = start.elapsed().as_nanos();
                        if *threads == 8 {
                            ns8 = elapsed;
                        }
                        kept = result.retained_events;
                        calls = result.full_calls;
                        fast = result.fast_probes;
                        certificate_tests = result.certificate.candidates_tested;
                        case_pass &= result.closure_1_minimal
                            && result.stop_reason.is_none()
                            && result.target.instance_key.as_ref() == Some(key);
                        let noise = store.order_index("NOISE").unwrap();
                        case_pass &= !result
                            .alive
                            .contains(*store.orders.unit.get(noise).unwrap() as usize);
                        let dir = base.join(format!("{shape}-{size}-{seed}-{defect:?}-{threads}"));
                        qlrs_bundle::write(&dir, &trace, &result).unwrap();
                        qlrs_bundle::verify_with_options(&dir, true).unwrap();
                        let mut files: Vec<_> = std::fs::read_dir(&dir)
                            .unwrap()
                            .map(|entry| {
                                let entry = entry.unwrap();
                                (
                                    entry.file_name().to_string_lossy().into_owned(),
                                    std::fs::read(entry.path()).unwrap(),
                                )
                            })
                            .collect();
                        files.sort_by(|left, right| left.0.cmp(&right.0));
                        log_bytes = files
                            .iter()
                            .find(|(name, _)| name == "reduction_log.jsonl")
                            .unwrap()
                            .1
                            .len();
                        if let Some(previous) = &canonical {
                            case_pass &= *previous == files;
                        } else {
                            canonical = Some(files);
                        }
                        std::fs::remove_dir_all(dir).unwrap();
                        let (first, last) = result.alive.bounds();
                        let mut contributing_units = std::collections::BTreeSet::new();
                        required = 0;
                        context = 0;
                        for event in 0..store.len() {
                            let seq = *store.spine.seq.get(event).unwrap();
                            if first.is_some_and(|first| seq <= first)
                                || last.is_some_and(|last| seq > last)
                                || store.spine.kind.get(event) != Some(&Kind::Filled)
                            {
                                continue;
                            }
                            let fill = *store.spine.payload_idx.get(event).unwrap() as usize;
                            let order = *store.fills.order.get(fill).unwrap() as usize;
                            let relevant = match key.invariant_id {
                                Invariant::I1 => true,
                                Invariant::I2 => store
                                    .orders
                                    .instrument
                                    .get(order)
                                    .and_then(|id| store.header.instruments.get(*id as usize))
                                    .is_some_and(|instrument| instrument.id == key.scope),
                                Invariant::I3 => store
                                    .orders
                                    .unit
                                    .get(order)
                                    .is_some_and(|unit| result.alive.contains(*unit as usize)),
                            };
                            if relevant {
                                required += 1;
                                contributing_units.insert(*store.orders.unit.get(order).unwrap());
                            }
                        }
                        if key.invariant_id == Invariant::I3 {
                            for identity in &result.certificate.kept_units {
                                contributing_units.insert(identity.id);
                            }
                        }
                        for event in 0..store.len() {
                            let seq = *store.spine.seq.get(event).unwrap();
                            if first.is_some_and(|first| seq < first)
                                || last.is_some_and(|last| seq > last)
                            {
                                continue;
                            }
                            if !matches!(
                                store.spine.kind.get(event),
                                Some(Kind::CashClaim | Kind::PositionClaim | Kind::Filled)
                            ) && store
                                .spine
                                .unit
                                .get(event)
                                .is_some_and(|id| contributing_units.contains(id))
                            {
                                context += 1;
                            }
                        }
                    }
                    let bound = required + 2 + context;
                    case_pass &=
                        minimum.map_or(kept <= bound, |minimum| kept <= minimum.saturating_mul(2));
                    case_pass &= (*size != 10_000 || (ns8 <= 1_000_000_000 && calls <= 200))
                        && (*size != 100_000 || ns8 <= 5_000_000_000);
                    if certificate_tests > 0 {
                        genuine += 1;
                    }
                    if !case_pass {
                        failures += 1;
                    }
                    total += 1;
                    println!(
                        "{{\"shape\":\"{shape}\",\"defect\":\"{defect:?}\",\"seed\":{seed},\"requested_events\":{size},\"actual_events\":{},\"kept_events\":{kept},\"case_bound\":{bound},\"exhaustive_minimum\":{},\"full_slots\":{calls},\"fast_probes\":{fast},\"certificate_candidates_tested\":{certificate_tests},\"wall_ns_8\":{ns8},\"log_bytes\":{log_bytes},\"pass\":{case_pass}}}",
                        store.len(),
                        minimum.map_or("null".into(), |value| value.to_string())
                    );
                }
            }
        }
    }
    std::fs::remove_dir(base).unwrap();
    println!(
        "{{\"summary\":true,\"cases\":{total},\"failures\":{failures},\"nonvacuous_cases\":{genuine}}}"
    );
    assert_eq!(failures, 0);
    assert!(genuine > 0);
}
// The gate is in testkit, whose runtime already owns rayon through its consumers.
fn rayon_pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
}
