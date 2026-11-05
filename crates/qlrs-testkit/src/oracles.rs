//! Independent integer oracles shared by libFuzzer and stable corpus replay.
use arbitrary::Unstructured;
use num_bigint::{BigInt, Sign};
use qlrs_io::{Limits, ParsedTrace};
use qlrs_num::{Fixed, NumericError, Rounding};

fn limits() -> Limits {
    Limits {
        max_line_bytes: 4096,
        max_events: 2000,
        max_interned: 1024,
        max_input_bytes: 1 << 20,
        ..Limits::default()
    }
}

/// One record canonicalizes idempotently, or produces a stable typed error.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn io_line(data: &[u8]) {
    match qlrs_io::canonical_record(data, limits()) {
        Ok(canonical) => assert_eq!(
            qlrs_io::canonical_record(&canonical, limits()).ok(),
            Some(canonical)
        ),
        Err(error) => assert!(
            error.code.starts_with("E-")
                || error.code.starts_with("U-")
                || error.code.starts_with("I-")
        ),
    }
}
/// Bounded complete traces have valid shapes or an error inside their input.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn io_trace(data: &[u8]) {
    match ParsedTrace::parse_with_limits(data.to_vec(), limits()) {
        Ok(trace) => assert!(trace.store.shape_valid()),
        Err(error) => {
            let lines = data.split_inclusive(|byte| *byte == b'\n').count().max(1);
            assert!(error.line <= lines, "{} > {lines}", error.line);
            assert!(
                error.code.starts_with("E-")
                    || error.code.starts_with("U-")
                    || error.code.starts_with("I-")
            );
        }
    }
}
fn power(scale: u8) -> BigInt {
    BigInt::from(10).pow(u32::from(scale))
}
fn narrow(value: BigInt) -> Result<i128, NumericError> {
    i128::try_from(value).map_err(|_| NumericError::Overflow)
}
fn quantize(value: BigInt, from: u8, to: u8, mode: Rounding) -> Result<i128, NumericError> {
    if from <= to {
        return narrow(value * power(to.saturating_sub(from)));
    }
    let divisor = power(from.saturating_sub(to));
    let mut quotient = &value / &divisor;
    // SPEC's final-overflow priority, also modeled by ci/numeric-oracle.py.
    narrow(quotient.clone())?;
    let remainder = &value % &divisor;
    if remainder != BigInt::from(0) {
        if mode == Rounding::Exact {
            return Err(NumericError::RoundingRequired);
        }
        let magnitude = if remainder.sign() == Sign::Minus {
            -remainder
        } else {
            remainder
        };
        let twice = magnitude * 2;
        let up = match mode {
            Rounding::HalfUp => twice >= divisor,
            Rounding::HalfEven => {
                twice > divisor || (twice == divisor && &quotient % 2 != BigInt::from(0))
            }
            _ => false,
        };
        if up {
            quotient += if value.sign() == Sign::Minus { -1 } else { 1 };
        }
    }
    narrow(quotient)
}
/// Full-width operations agree with big integers and documented alignment bounds.
/// Arbitrary division is not exposed by Fixed; rescale covers its power-of-ten
/// division, and negation uses zero minus the operand at the same scale.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn num_ops(data: &[u8]) {
    let Ok((left, right, ls, rs, target, op, mode)) =
        Unstructured::new(data).arbitrary::<(i128, i128, u8, u8, u8, u8, u8)>()
    else {
        return;
    };
    let (ls, rs, target) = (ls % 19, rs % 19, target % 19);
    let left_fixed = Fixed::new(left, ls).unwrap_or_else(|e| panic!("{e}"));
    let right_fixed = Fixed::new(right, rs).unwrap_or_else(|e| panic!("{e}"));
    let mode = [
        Rounding::Exact,
        Rounding::HalfEven,
        Rounding::HalfUp,
        Rounding::Down,
    ]
    .get(usize::from(mode % 4))
    .copied()
    .unwrap_or(Rounding::Exact);
    let common = ls.max(rs);
    let (actual, expected, scale) = match op % 7 {
        0 | 1 => {
            let aligned_left = BigInt::from(left) * power(common.saturating_sub(ls));
            let aligned_right = BigInt::from(right) * power(common.saturating_sub(rs));
            let expected = if narrow(aligned_left.clone()).is_err()
                || narrow(aligned_right.clone()).is_err()
            {
                Err(NumericError::Overflow)
            } else {
                narrow(if op % 7 == 0 {
                    aligned_left + aligned_right
                } else {
                    aligned_left - aligned_right
                })
            };
            (
                if op % 7 == 0 {
                    left_fixed.checked_add(right_fixed)
                } else {
                    left_fixed.checked_sub(right_fixed)
                },
                expected,
                common,
            )
        }
        2 => (
            left_fixed.checked_mul(right_fixed),
            if ls.saturating_add(rs) > 18 {
                Err(NumericError::UnsupportedScale)
            } else {
                narrow(BigInt::from(left) * BigInt::from(right))
            },
            ls.saturating_add(rs),
        ),
        3 => (
            left_fixed.mul_rescale(right_fixed, target, mode),
            quantize(
                BigInt::from(left) * BigInt::from(right),
                ls.saturating_add(rs),
                target,
                mode,
            ),
            target,
        ),
        4 => (
            left_fixed.rescale(target, mode),
            quantize(BigInt::from(left), ls, target, mode),
            target,
        ),
        5 => {
            assert_eq!(
                left_fixed.numeric_cmp(right_fixed),
                (BigInt::from(left) * power(rs)).cmp(&(BigInt::from(right) * power(ls)))
            );
            return;
        }
        _ => (
            Fixed::new(0, ls).and_then(|zero| zero.checked_sub(left_fixed)),
            narrow(-BigInt::from(left)),
            ls,
        ),
    };
    assert_eq!(
        actual.map(|value| (value.raw(), value.scale())),
        expected.map(|raw| (raw, scale))
    );
}
/// Literal acceptance agrees with integer parsing; formatting accepted literals
/// round-trips their raw value and scale without floating-point conversion.
/// # Panics
/// Panics when the implementation disagrees with the independent oracle.
pub fn num_decimal(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let bounded = limits().number(text, 1);
    if text.len() > 40 {
        assert_eq!(bounded.err().map(|e| e.code), Some("E-LIMIT-NUMBER-LEN"));
        return;
    }
    if text
        .split_once('.')
        .is_some_and(|(_, tail)| tail.len() > 18)
    {
        assert_eq!(bounded.err().map(|e| e.code), Some("E-SCALE"));
        return;
    }
    assert!(bounded.is_ok());
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let syntax = !integer.is_empty()
        && integer.bytes().all(|b| b.is_ascii_digit())
        && fraction.bytes().all(|b| b.is_ascii_digit())
        && (!unsigned.contains('.') || !fraction.is_empty());
    let expected = if syntax && fraction.len() <= 18 {
        BigInt::parse_bytes(format!("{integer}{fraction}").as_bytes(), 10)
            .and_then(|value| {
                i128::try_from(if text.starts_with('-') { -value } else { value }).ok()
            })
            .zip(u8::try_from(fraction.len()).ok())
    } else {
        None
    };
    let actual = Fixed::parse(text, 18);
    assert_eq!(actual.as_ref().ok().map(|v| (v.raw(), v.scale())), expected);
    if let Ok(value) = actual {
        assert_eq!(Fixed::parse(&value.to_string(), 18), Ok(value));
    }
}

fn pool(threads: usize) -> &'static rayon::ThreadPool {
    use std::sync::OnceLock;
    static POOLS: [OnceLock<rayon::ThreadPool>; 8] = [const { OnceLock::new() }; 8];
    POOLS
        .get(threads.saturating_sub(1))
        .unwrap_or_else(|| panic!("invalid worker count"))
        .get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap_or_else(|e| panic!("{e}"))
        })
}

/// Chunk sizes and worker scheduling cannot change columns, IDs or first errors.
/// # Panics
/// Panics if sequential and parallel ingestion disagree.
pub fn io_chunk_diff(data: &[u8]) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let [low, high, workers, bytes @ ..] = data else {
        return;
    };
    let chunk = usize::from(u16::from_le_bytes([*low, *high])) + 1;
    let threads = usize::from(*workers % 8);
    let path = std::env::temp_dir().join(format!(
        "qlrs-fuzz-chunk-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    qlrs_io::atomic_file(&path, |f| std::io::Write::write_all(f, bytes))
        .unwrap_or_else(|e| panic!("{e}"));
    let left = ParsedTrace::read_chunked(&path, Some(limits()), 1);
    let right =
        pool(threads + 1).install(|| ParsedTrace::read_chunked(&path, Some(limits()), chunk));
    std::fs::remove_file(path).unwrap_or_else(|e| panic!("{e}"));
    match (left, right) {
        (Ok(left), Ok(right)) => {
            assert_eq!(left.store, right.store);
            assert_eq!(left.source_digest(), right.source_digest());
            for event in 0..left.store.len() {
                assert_eq!(left.source_ref(event), right.source_ref(event));
            }
        }
        (Err(left), Err(right)) => assert_eq!(
            (left.kind, left.code, left.line, left.message),
            (right.kind, right.code, right.line, right.message)
        ),
        (left, right) => panic!("chunk divergence: {left:?} != {right:?}"),
    }
}

#[derive(Debug, Clone, Copy)]
struct Case {
    seed: u64,
    shape: u8,
    size: u8,
    defect: u8,
    bits: u64,
}
fn case(data: &[u8]) -> Option<Case> {
    let (seed, shape, size, defect, bits) = Unstructured::new(data)
        .arbitrary::<(u64, u8, u8, u8, u64)>()
        .ok()?;
    Some(Case {
        seed,
        shape,
        size,
        defect,
        bits,
    })
}
fn fixture(case: Case, defect: Option<crate::Defect>) -> crate::Fixture {
    let orders = 16 + usize::from(case.size % 49);
    let simple = case.shape % 9 == 0
        || matches!(
            defect,
            Some(crate::Defect::D12 | crate::Defect::D13 | crate::Defect::D14)
        );
    if simple {
        return crate::generate(orders, case.seed, defect.map(|d| (d, orders / 2)))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    let shape = crate::SHAPES
        .get(usize::from(case.shape % 9))
        .copied()
        .unwrap_or("S2");
    let mut config =
        crate::shape_config(shape, case.seed, orders).unwrap_or_else(|e| panic!("{e}"));
    // S6/S7 deliberately inject errors; their scheduling is useful for clean
    // fuzzing, but their error switches are not a clean-input oracle.
    config.earlier_cash_error = false;
    config.dense_violations = false;
    let index = if defect == Some(crate::Defect::D04) && shape == "S8" {
        config.fifo_ladder
    } else {
        orders / 2
    };
    crate::generate_rich(config, defect.map(|d| (d, index))).unwrap_or_else(|e| panic!("{e}"))
}
fn prepared(bytes: Vec<u8>) -> ParsedTrace {
    let mut trace =
        ParsedTrace::parse_with_limits(bytes, limits()).unwrap_or_else(|e| panic!("{e}"));
    qlrs_check::Checker::prepare(&mut trace.store);
    trace
}
fn check(trace: &ParsedTrace, mask: &qlrs_schema::AliveMask) -> qlrs_check::Report {
    qlrs_check::Checker::check(&trace.store, mask, None, false)
}
fn all(trace: &ParsedTrace) -> qlrs_schema::AliveMask {
    qlrs_schema::AliveMask::all(trace.store.units.len())
}
fn defect(case: Case) -> crate::Defect {
    crate::DEFECTS
        .get(usize::from(case.defect % 14))
        .copied()
        .unwrap_or(crate::Defect::D03)
}
fn violating(case: Case) -> crate::Defect {
    crate::DEFECTS
        .get(usize::from(case.defect % 11))
        .copied()
        .unwrap_or(crate::Defect::D03)
}
fn first_violation(report: &qlrs_check::Report) -> Option<u64> {
    report
        .findings
        .iter()
        .filter(|f| f.status == qlrs_schema::Status::Violation)
        .map(|f| f.window.end_seq)
        .min()
}

/// Clean lifecycle generators cannot create a financial accusation.
/// # Panics
/// Panics on a generated false positive or worker-dependent result.
pub fn check_clean(data: &[u8]) {
    let Some(case) = case(data) else {
        return;
    };
    let trace = prepared(fixture(case, None).bytes);
    let report = check(&trace, &all(&trace));
    assert!(matches!(
        report.status,
        qlrs_schema::Status::Consistent | qlrs_schema::Status::Inconclusive
    ));
    for threads in [1, 4] {
        assert_eq!(
            report,
            pool(threads).install(|| check(&trace, &all(&trace)))
        );
    }
}

/// Catalogue classification, actual injection location and supported-prefix law.
/// # Panics
/// Panics on a classification or first-violation disagreement.
pub fn check_injected(data: &[u8]) {
    use qlrs_schema::Status;
    let Some(case) = case(data) else {
        return;
    };
    let defect = defect(case);
    let input = fixture(case, Some(defect));
    let clean = fixture(case, None);
    let trace = prepared(input.bytes);
    let report = check(&trace, &all(&trace));
    assert_eq!(report.status, defect.expected());
    if defect.expected() == Status::Violation {
        let invariant = match defect {
            crate::Defect::D06 => qlrs_check::Invariant::I2,
            crate::Defect::D07 | crate::Defect::D08 | crate::Defect::D09 | crate::Defect::D11 => {
                qlrs_check::Invariant::I3
            }
            _ => qlrs_check::Invariant::I1,
        };
        assert!(report.keys().any(|k| k.invariant_id == invariant));
        #[derive(serde::Deserialize)]
        struct Sequence {
            seq: u64,
        }
        let injection = trace
            .source
            .split_inclusive(|b| *b == b'\n')
            .zip(clean.bytes.split_inclusive(|b| *b == b'\n'))
            .find(|(left, right)| left != right)
            .and_then(|(line, _)| qlrs_io::decode::<Sequence>(line).ok())
            .map_or(0, |v| v.seq);
        assert!(first_violation(&report).is_some_and(|seq| seq >= injection));
    }
    // Unsupported tails are outside the supported-prefix property (rebase).
    if report.status != Status::Unsupported {
        let index = usize::try_from(case.bits % trace.store.len() as u64).unwrap_or(0);
        let end = trace.store.spine.seq.get(index).copied();
        let prefix = check(&trace, &all(&trace).with_range(None, end));
        if let Some(seq) = first_violation(&prefix) {
            assert!(first_violation(&report).is_some_and(|full| full <= seq));
        }
        if report.status == Status::Consistent {
            assert!(first_violation(&prefix).is_none());
        }
    }
}

/// Dense, selected and checkpoint replay preserve every observable result.
/// # Panics
/// Panics if a replay entry point diverges.
pub fn check_dense_vs_selected(data: &[u8]) {
    let Some(case) = case(data) else {
        return;
    };
    let trace = prepared(fixture(case, Some(violating(case))).bytes);
    let store = &trace.store;
    let events: Vec<_> = (0..store.len()).collect();
    assert_eq!(
        check(&trace, &all(&trace)),
        qlrs_check::Checker::check_selected(store, &all(&trace), None, false, &events)
    );
    let mut mask = qlrs_schema::AliveMask::empty(store.units.len());
    for unit in 0..store.units.len() {
        mask.set(
            unit,
            case.bits
                .rotate_right(u32::try_from(unit % 64).unwrap_or(0))
                & 1
                != 0,
        );
    }
    let selected: Vec<_> = (0..store.len())
        .filter(|event| mask.contains_event(store, *event))
        .collect();
    assert_eq!(
        check(&trace, &mask),
        qlrs_check::Checker::check_selected(store, &mask, None, false, &selected)
    );
    let claims: Vec<_> = store
        .spine
        .seq
        .iter()
        .zip(&store.spine.kind)
        .filter(|(_, kind)| {
            matches!(
                kind,
                qlrs_schema::Kind::CashClaim | qlrs_schema::Kind::PositionClaim
            )
        })
        .map(|(seq, _)| *seq)
        .collect();
    if let Some(cut) = claims.get(usize::from(case.size) % claims.len()) {
        let end = store.spine.seq.last().copied().unwrap_or(*cut);
        let cp = qlrs_check::Checker::checkpoint_at_claim(
            store,
            *cut,
            end,
            None,
            &trace.source_digest(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let before =
            qlrs_ledger::Ledger::replay(store, &all(&trace).with_range(None, Some(*cut)), None)
                .unwrap_or_else(|e| panic!("{e}"));
        let initial =
            qlrs_ledger::Ledger::initial(store, Some(&cp)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (before.cash, before.positions),
            (initial.cash, initial.positions)
        );
        let full = qlrs_ledger::Ledger::replay(store, &all(&trace), None)
            .unwrap_or_else(|e| panic!("{e}"));
        let suffix = qlrs_ledger::Ledger::replay(
            store,
            &all(&trace).with_range(cut.checked_add(1), None),
            Some(&cp),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((full.cash, full.positions), (suffix.cash, suffix.positions));
    }
}

/// Bounded deterministic reduction reproduces its exact scoped target and bytes.
/// # Panics
/// Panics on an unsound witness, certificate or nondeterministic reduction.
pub fn reduce_sound(data: &[u8]) {
    let Some(case) = case(data) else {
        return;
    };
    let input = fixture(case, Some(violating(case)));
    let trace = prepared(input.bytes);
    let limits = qlrs_reduce::ReductionLimits {
        max_calls: 64,
        time_limit: std::time::Duration::MAX,
    };
    let reduce = || {
        qlrs_reduce::reduce_with_limits(
            &trace.store,
            &trace.source_digest(),
            None,
            input.target_seq,
            limits,
        )
        .unwrap_or_else(|e| panic!("{case:?}: {e}"))
    };
    let left = reduce();
    let right = reduce();
    let mut bytes = Vec::new();
    trace
        .write_kept(&left.alive, &mut bytes)
        .unwrap_or_else(|e| panic!("{case:?}: {e}"));
    let mut again = Vec::new();
    trace
        .write_kept(&right.alive, &mut again)
        .unwrap_or_else(|e| panic!("{case:?}: {e}"));
    assert_eq!(bytes, again);
    assert_eq!(
        qlrs_io::encode(&left.certificate).ok(),
        qlrs_io::encode(&right.certificate).ok()
    );
    assert!(left.retained_events <= trace.store.len());
    let mut witness = ParsedTrace::parse(bytes).unwrap_or_else(|e| panic!("{case:?}: {e}"));
    qlrs_check::Checker::prepare_with_checkpoint(&mut witness.store, left.checkpoint.as_ref())
        .unwrap_or_else(|e| panic!("{case:?}: {e}"));
    let key = left
        .target
        .instance_key
        .as_ref()
        .unwrap_or_else(|| panic!("missing key"));
    let report = qlrs_check::Checker::check_witness(
        &witness.store,
        &all(&witness),
        left.checkpoint.as_ref(),
        key,
    );
    assert_eq!(report.status, qlrs_schema::Status::Violation);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.instance_key.as_ref() == Some(key) && f.class_fp == left.target.class_fp)
    );
    assert!(!report.findings.iter().any(|f| matches!(
        f.status,
        qlrs_schema::Status::Unsupported | qlrs_schema::Status::Inconclusive
    )));
    for (event, seq) in witness.store.spine.seq.iter().enumerate() {
        let original = trace
            .store
            .spine
            .seq
            .binary_search(seq)
            .unwrap_or_else(|e| panic!("{case:?}: {e}"));
        assert_eq!(
            witness
                .source
                .get(witness.line_range(event + 1).unwrap_or(0..0)),
            trace
                .source
                .get(trace.line_range(original + 1).unwrap_or(0..0))
        );
    }
    qlrs_reduce::recheck_certificate(&witness.store, left.checkpoint.as_ref(), &left.certificate)
        .unwrap_or_else(|e| panic!("{case:?}: {e}"));
}

/// Linear residuals agree with the full checker on dependency-closed subsets.
/// Admission and lifecycle validity remain the full checker's responsibility.
/// # Panics
/// Panics on an algebra/checker disagreement.
pub fn eval_vs_checker(data: &[u8]) {
    use qlrs_reduce::FastPredicate;
    let Some(case) = case(data) else {
        return;
    };
    let input = fixture(
        case,
        Some(if case.defect % 2 == 0 {
            crate::Defect::D03
        } else {
            crate::Defect::D06
        }),
    );
    let trace = prepared(input.bytes);
    let store = &trace.store;
    let original = check(&trace, &all(&trace));
    let finding = original
        .at_seq(input.target_seq.unwrap_or(0))
        .unwrap_or_else(|| panic!("missing target"));
    let key = finding
        .instance_key
        .as_ref()
        .unwrap_or_else(|| panic!("missing key"));
    let predicate = qlrs_reduce::ReductionPredicate::new(
        store,
        key.clone(),
        &original,
        None,
        &trace.source_digest(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut mask = predicate.candidate_mask(&all(&trace));
    let cp = predicate
        .candidate_checkpoint(&mask)
        .unwrap_or_else(|e| panic!("{e}"));
    let graph = qlrs_reduce::UnitGraph::build_with_checkpoint(store, cp.as_ref(), &mask)
        .unwrap_or_else(|e| panic!("{e}"));
    let pins = qlrs_reduce::pinned_units(store, finding).unwrap_or_else(|e| panic!("{e}"));
    let table =
        qlrs_reduce::ContributionTable::new(store, &mask, key).unwrap_or_else(|e| panic!("{e}"));
    for (index, id) in table.units.iter().enumerate() {
        if case
            .bits
            .rotate_right(u32::try_from(index % 64).unwrap_or(0))
            & 1
            == 0
        {
            continue;
        }
        let closure = graph.closure(&[*id as usize]);
        if !closure.iter().any(|id| pins.contains(*id)) {
            for id in closure {
                mask.set(id, false);
            }
        }
    }
    if let Some(expected) = table.residual(&mask) {
        let cp = predicate
            .candidate_checkpoint(&mask)
            .unwrap_or_else(|e| panic!("{e}"));
        let report = qlrs_check::Checker::check_witness(store, &mask, cp.as_ref(), key);
        assert!(!report.findings.iter().any(|f| matches!(
            f.status,
            qlrs_schema::Status::Unsupported | qlrs_schema::Status::Inconclusive
        )));
        let actual = report
            .keys()
            .find(|k| {
                k.invariant_id == key.invariant_id
                    && k.anchor_claim_original_seq == key.anchor_claim_original_seq
            })
            .map_or(0, |k| {
                k.residual_raw
                    .parse::<i128>()
                    .unwrap_or_else(|e| panic!("{e}"))
            });
        assert_eq!(expected, actual);
    }
}
