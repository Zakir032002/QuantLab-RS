//! Local financial invariants, lifecycle legality and non-accusatory verdicts.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic, clippy::arithmetic_side_effects)]
#![allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
use qlrs_ledger::{ExpectedState, Ledger};
use qlrs_num::{Fixed, NumericError, Rounding};
use qlrs_schema::{AliveMask, Kind, OrderState, Status, SyntheticCheckpoint, TraceStore};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Invariant identifier, stable across evidence versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Invariant {
    /// Local cash conservation.
    I1,
    /// Position equals independent fill sum.
    I2,
    /// Lifecycle legality and trade uniqueness.
    I3,
}
impl Invariant {
    /// Canonical invariant name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I1 => "I1",
            Self::I2 => "I2",
            Self::I3 => "I3",
        }
    }
}
/// Exact identity preserved by reduction; strings keep the raw integer lossless.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceKey {
    #[doc = "invariant id."]
    pub invariant_id: Invariant,
    #[doc = "sub code."]
    pub sub_code: String,
    #[doc = "account."]
    pub account: String,
    #[doc = "scope."]
    pub scope: String,
    #[doc = "anchor claim original seq."]
    pub anchor_claim_original_seq: u64,
    #[doc = "residual raw."]
    pub residual_raw: String,
    #[doc = "residual scale."]
    pub residual_scale: u8,
}
/// Inclusive anchor and exclusive predecessor of the evidence window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    #[doc = "start seq."]
    pub start_seq: Option<u64>,
    #[doc = "end seq."]
    pub end_seq: u64,
}
/// A contradiction or an explicit limitation. Relations are hints, not diagnoses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    #[doc = "status."]
    pub status: Status,
    #[doc = "instance key."]
    pub instance_key: Option<InstanceKey>,
    #[doc = "class fp."]
    pub class_fp: Option<String>,
    #[doc = "residual."]
    pub residual: Option<String>,
    #[doc = "expected."]
    pub expected: Option<String>,
    #[doc = "observed."]
    pub observed: Option<String>,
    #[doc = "window."]
    pub window: Window,
    #[doc = "relation."]
    pub relation: String,
    #[doc = "reason."]
    pub reason: Option<String>,
}
/// Deterministic verification report with an independently derived final summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    #[doc = "status."]
    pub status: Status,
    #[doc = "findings."]
    pub findings: Vec<Finding>,
    #[doc = "expected cash."]
    pub expected_cash: String,
    #[doc = "expected positions."]
    pub expected_positions: Vec<String>,
}
impl Report {
    /// Iterate only fully evidenced violation instance keys.
    pub fn keys(&self) -> impl Iterator<Item = &InstanceKey> {
        self.findings.iter().filter_map(|f| f.instance_key.as_ref())
    }
    /// Find a violation at an original anchor sequence.
    #[must_use]
    pub fn at_seq(&self, seq: u64) -> Option<&Finding> {
        self.findings
            .iter()
            .find(|f| f.status == Status::Violation && f.window.end_seq == seq)
    }
}
/// Table-driven transitions for the supported lifecycle subset.
pub const TRANSITIONS: &[(OrderState, Kind, OrderState)] = &[
    (OrderState::Missing, Kind::Submitted, OrderState::Submitted),
    (OrderState::Submitted, Kind::Accepted, OrderState::Accepted),
    (OrderState::Submitted, Kind::Rejected, OrderState::Rejected),
    (OrderState::Accepted, Kind::Filled, OrderState::Partial),
    (OrderState::Partial, Kind::Filled, OrderState::Partial),
    (OrderState::Accepted, Kind::Canceled, OrderState::Canceled),
    (OrderState::Partial, Kind::Canceled, OrderState::Canceled),
];
/// Invariant evaluator, independent of JSON I/O.
pub struct Checker;
impl Checker {
    /// Prepare independent numeric deltas without introducing an I/O dependency.
    pub fn prepare(store: &mut TraceStore) {
        Ledger::prepare(store);
    }
    /// Stable Part 2 entry point for complete traces.
    #[must_use]
    pub fn evaluate(store: &TraceStore, _expected: &ExpectedState) -> Report {
        Self::check(store, &AliveMask::all(store.units.len()), None, false)
    }
    /// Check a candidate by replaying only complete live units.
    #[must_use]
    // SAFETY-BOUNDS: shape and reference validation precedes the hot loop; the
    // safe I/O builder guarantees these indices. No unchecked memory access.
    #[allow(clippy::indexing_slicing)]
    pub fn check(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
        partial: bool,
    ) -> Report {
        let mut report = Report {
            status: Status::Consistent,
            findings: Vec::new(),
            expected_cash: String::new(),
            expected_positions: Vec::new(),
        };
        if !store.prepared || !store.shape_valid() || store.delta_cash.len() != store.len() {
            report.findings.push(limitation(
                Status::Inconclusive,
                0,
                "unprepared-or-invalid-store",
            ));
            return finish(report, partial);
        }
        for issue in &store.issues {
            report.findings.push(limitation(
                issue.status,
                issue.seq.unwrap_or(0),
                &issue.reason,
            ));
        }
        if store
            .issues
            .iter()
            .any(|issue| issue.seq.is_none() || (issue.status == Status::Unsupported && !partial))
        {
            return finish(report, partial);
        }
        let uncertain_after = store.issues.iter().filter_map(|issue| issue.seq).min();
        if checkpoint.is_some_and(|cp| {
            store
                .spine
                .unit
                .iter()
                .position(|u| alive.contains(*u as usize))
                .and_then(|e| store.spine.seq.get(e))
                .is_some_and(|seq| cp.cut_seq > *seq)
        }) {
            report.findings.push(limitation(
                Status::Inconclusive,
                0,
                "checkpoint-cut-after-first-kept-event",
            ));
            return finish(report, partial);
        }
        let mut state = match Ledger::initial(store, checkpoint) {
            Ok(s) => s,
            Err(reason) => {
                report
                    .findings
                    .push(limitation(Status::Inconclusive, 0, &reason));
                return finish(report, partial);
            }
        };
        let mut states = vec![OrderState::Missing; store.orders.id.len()];
        let mut totals = vec![0_i128; store.orders.id.len()];
        let mut first = vec![None; store.orders.id.len()];
        let mut trades = HashSet::with_capacity_and_hasher(
            store.fills.order.len(),
            foldhash::fast::RandomState::default(),
        );
        let mut base = state.cash;
        let mut window_delta = 0_i128;
        let mut previous = None;
        let mut window_start = 0_usize;
        let mut cash_claim_count = 0_usize;
        let mut filled_instruments = vec![false; state.positions.len()];
        let mut claimed_instruments = vec![false; state.positions.len()];
        let mut financial = true;
        for event in 0..store.len() {
            if !alive.contains(store.spine.unit[event] as usize) {
                continue;
            }
            let kind = store.spine.kind[event];
            let payload = store.spine.payload_idx[event] as usize;
            let seq = store.spine.seq[event];
            if uncertain_after.is_some_and(|cut| seq >= cut) {
                break;
            }
            if matches!(
                kind,
                Kind::Submitted | Kind::Accepted | Kind::Rejected | Kind::Filled | Kind::Canceled
            ) {
                let order = if kind == Kind::Filled {
                    store
                        .fills
                        .order
                        .get(payload)
                        .copied()
                        .unwrap_or(qlrs_schema::NONE) as usize
                } else {
                    payload
                };
                if order >= states.len() {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        seq,
                        "invalid-order-index",
                    ));
                    financial = false;
                    continue;
                }
                if first[order].is_none() {
                    first[order] = Some(seq);
                }
                let state_before = states[order];
                let mut codes = Vec::new();
                let terminal = matches!(
                    state_before,
                    OrderState::Filled | OrderState::Canceled | OrderState::Rejected
                );
                if terminal {
                    codes.push("event-after-terminal");
                } else if kind == Kind::Filled && state_before == OrderState::Submitted {
                    codes.push("fill-before-accept");
                } else if state_before == OrderState::Missing && kind != Kind::Submitted {
                    codes.push("unknown-order");
                } else if !TRANSITIONS
                    .iter()
                    .any(|(s, k, _)| *s == state_before && *k == kind)
                {
                    codes.push("illegal-transition");
                }
                let instrument = store.orders.instrument[order] as usize;
                if kind == Kind::Filled && store.header.instruments.get(instrument).is_none() {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        seq,
                        "missing-order-metadata",
                    ));
                    financial = false;
                }
                let mut excess = 0_i128;
                if kind == Kind::Filled {
                    if instrument < filled_instruments.len() {
                        filled_instruments[instrument] = true;
                    }
                    if store.header.instruments.get(instrument).is_some()
                        && !trades.insert((instrument, store.fills.trade[payload]))
                    {
                        codes.push("duplicate-trade-id");
                    }
                    let qty = store.fill_qty.get(payload).copied().unwrap_or(0);
                    if let Some(total) = totals[order].checked_add(qty) {
                        totals[order] = total;
                    } else {
                        report.findings.push(limitation(
                            Status::Inconclusive,
                            seq,
                            "numeric-overflow",
                        ));
                        financial = false;
                    }
                    if let Some(metadata) = store.header.instruments.get(instrument) {
                        match Fixed::new(store.orders.qty_raw[order], store.orders.qty_scale[order])
                            .and_then(|q| q.rescale(metadata.qty_precision, Rounding::Exact))
                        {
                            Ok(quantity) => {
                                if totals[order] > quantity.raw() {
                                    codes.push("overfill");
                                    excess = totals[order].checked_sub(quantity.raw()).unwrap_or(0);
                                }
                            }
                            Err(error) => {
                                report.findings.push(limitation(
                                    Status::Inconclusive,
                                    seq,
                                    &error.to_string(),
                                ));
                                financial = false;
                            }
                        }
                    }
                }
                for code in codes {
                    let scale = if code == "overfill" {
                        store
                            .header
                            .instruments
                            .get(instrument)
                            .map_or(0, |i| i.qty_precision)
                    } else {
                        0
                    };
                    let residual = if code == "overfill" { excess } else { 0 };
                    let scope = store
                        .header
                        .instruments
                        .get(instrument)
                        .map_or("<unknown>", |i| i.id.as_str());
                    let pattern = order_pattern(store, alive, order, event);
                    report.findings.push(violation(
                        store,
                        Invariant::I3,
                        code,
                        scope,
                        seq,
                        residual,
                        scale,
                        None,
                        None,
                        Window {
                            start_seq: first[order],
                            end_seq: seq,
                        },
                        &pattern,
                        "other",
                    ));
                }
                if let Some((_, _, next)) = TRANSITIONS
                    .iter()
                    .find(|(s, k, _)| *s == state_before && *k == kind)
                {
                    states[order] = *next;
                }
                if kind == Kind::Filled && !terminal {
                    let complete = store
                        .header
                        .instruments
                        .get(instrument)
                        .and_then(|metadata| {
                            Fixed::new(store.orders.qty_raw[order], store.orders.qty_scale[order])
                                .and_then(|q| q.rescale(metadata.qty_precision, Rounding::Exact))
                                .ok()
                        })
                        .is_some_and(|quantity| totals[order] >= quantity.raw());
                    states[order] = if complete {
                        OrderState::Filled
                    } else {
                        OrderState::Partial
                    };
                }
            }
            if kind == Kind::Filled && financial {
                if let Err(error) = state.apply(store, event) {
                    report
                        .findings
                        .push(limitation(error.status, seq, error.reason));
                    financial = false;
                }
                if let Some(sum) = window_delta.checked_add(store.delta_cash[event]) {
                    window_delta = sum;
                } else {
                    report
                        .findings
                        .push(limitation(Status::Inconclusive, seq, "numeric-overflow"));
                    financial = false;
                }
            }
            if kind == Kind::CashClaim {
                cash_claim_count = cash_claim_count.saturating_add(1);
                if financial {
                    let observed = Fixed::new(
                        store.cash_claims.raw[payload],
                        store.cash_claims.scale[payload],
                    )
                    .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact));
                    let expected = base
                        .checked_add(window_delta)
                        .ok_or(NumericError::Overflow)
                        .and_then(|raw| Fixed::new(raw, store.cash_scale))
                        .and_then(|v| {
                            v.rescale(
                                store.header.currencies.first().map_or(0, |c| c.precision),
                                store.header.model.engine_cash_rounding.numeric(),
                            )
                        })
                        .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact));
                    match (observed, expected) {
                        (Ok(observed), Ok(expected)) => {
                            if let Some(residual) = observed.raw().checked_sub(expected.raw()) {
                                if residual != 0 {
                                    let relation = relation(
                                        store,
                                        alive,
                                        window_start,
                                        event,
                                        residual,
                                        expected.raw(),
                                        observed.raw(),
                                    );
                                    let pattern = window_pattern(store, alive, window_start, event);
                                    report.findings.push(violation(
                                        store,
                                        Invariant::I1,
                                        "cash-conservation",
                                        &store.header.model.base_currency,
                                        seq,
                                        residual,
                                        store.cash_scale,
                                        Some(expected.to_string()),
                                        Some(observed_text(
                                            store,
                                            store.cash_claims.text[payload],
                                            store.cash_claims.raw[payload],
                                            store.cash_claims.scale[payload],
                                        )),
                                        Window {
                                            start_seq: previous,
                                            end_seq: seq,
                                        },
                                        &pattern,
                                        &relation,
                                    ));
                                }
                            } else {
                                report.findings.push(limitation(
                                    Status::Inconclusive,
                                    seq,
                                    "numeric-overflow",
                                ));
                                financial = false;
                            }
                            base = observed.raw();
                        }
                        (Err(error), _) | (_, Err(error)) => {
                            report.findings.push(limitation(
                                Status::Inconclusive,
                                seq,
                                &error.to_string(),
                            ));
                            financial = false;
                        }
                    }
                }
                previous = Some(seq);
                window_delta = 0;
                window_start = event.saturating_add(1);
            }
            if kind == Kind::PositionClaim {
                let instrument = store.position_claims.instrument[payload] as usize;
                if instrument >= state.positions.len() {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        seq,
                        "invalid-instrument-index",
                    ));
                    continue;
                }
                claimed_instruments[instrument] = true;
                if financial {
                    let metadata = &store.header.instruments[instrument];
                    let scale = metadata.qty_precision;
                    match Fixed::new(
                        store.position_claims.raw[payload],
                        store.position_claims.scale[payload],
                    )
                    .and_then(|v| v.rescale(scale, Rounding::Exact))
                    {
                        Ok(observed) => {
                            if let Some(residual) =
                                observed.raw().checked_sub(state.positions[instrument])
                            {
                                if residual != 0 {
                                    report.findings.push(violation(
                                        store,
                                        Invariant::I2,
                                        "position-sum",
                                        &metadata.id,
                                        seq,
                                        residual,
                                        scale,
                                        Fixed::new(state.positions[instrument], scale)
                                            .ok()
                                            .map(|v| v.to_string()),
                                        Some(observed_text(
                                            store,
                                            store.position_claims.text[payload],
                                            store.position_claims.raw[payload],
                                            store.position_claims.scale[payload],
                                        )),
                                        Window {
                                            start_seq: None,
                                            end_seq: seq,
                                        },
                                        &window_pattern(store, alive, 0, event),
                                        "other",
                                    ));
                                }
                            } else {
                                report.findings.push(limitation(
                                    Status::Inconclusive,
                                    seq,
                                    "numeric-overflow",
                                ));
                            }
                        }
                        Err(error) => report.findings.push(limitation(
                            Status::Inconclusive,
                            seq,
                            &error.to_string(),
                        )),
                    }
                }
            }
        }
        if filled_instruments.iter().any(|v| *v) && cash_claim_count == 0 {
            report
                .findings
                .push(limitation(Status::Inconclusive, 0, "missing-account-claim"));
        }
        for (filled, claimed) in filled_instruments.iter().zip(&claimed_instruments) {
            if *filled && !claimed {
                report.findings.push(limitation(
                    Status::Inconclusive,
                    0,
                    "missing-position-claim",
                ));
            }
        }
        report.expected_cash = Fixed::new(state.cash, store.cash_scale)
            .map_or_else(|_| String::new(), |v| v.to_string());
        report.expected_positions = state
            .positions
            .iter()
            .zip(&store.header.instruments)
            .map(|(raw, i)| {
                Fixed::new(*raw, i.qty_precision).map_or_else(|_| String::new(), |v| v.to_string())
            })
            .collect();
        finish(report, partial)
    }
}
fn limitation(status: Status, seq: u64, reason: &str) -> Finding {
    Finding {
        status,
        instance_key: None,
        class_fp: None,
        residual: None,
        expected: None,
        observed: None,
        window: Window {
            start_seq: None,
            end_seq: seq,
        },
        relation: "other".into(),
        reason: Some(reason.into()),
    }
}
fn finish(mut report: Report, partial: bool) -> Report {
    report.status = if report
        .findings
        .iter()
        .any(|f| f.status == Status::Unsupported)
    {
        Status::Unsupported
    } else if report
        .findings
        .iter()
        .any(|f| f.status == Status::Violation)
    {
        Status::Violation
    } else if report
        .findings
        .iter()
        .any(|f| f.status == Status::Inconclusive)
    {
        Status::Inconclusive
    } else {
        Status::Consistent
    };
    if report.status == Status::Unsupported && !partial {
        report.findings.retain(|f| f.status != Status::Violation);
    }
    report
}
fn violation(
    store: &TraceStore,
    invariant: Invariant,
    code: &str,
    scope: &str,
    seq: u64,
    raw: i128,
    scale: u8,
    expected: Option<String>,
    observed: Option<String>,
    window: Window,
    pattern: &[(Kind, u64)],
    relation: &str,
) -> Finding {
    let key = InstanceKey {
        invariant_id: invariant,
        sub_code: code.into(),
        account: store.account().into(),
        scope: scope.into(),
        anchor_claim_original_seq: seq,
        residual_raw: raw.to_string(),
        residual_scale: scale,
    };
    let sign = if raw < 0 { -1 } else { i8::from(raw > 0) };
    Finding {
        status: Status::Violation,
        instance_key: Some(key),
        class_fp: Some(class_fingerprint(invariant, code, pattern, sign, relation)),
        residual: Fixed::new(raw, scale).ok().map(|v| v.to_string()),
        expected,
        observed,
        window,
        relation: relation.into(),
        reason: None,
    }
}
/// Canonical fingerprint bytes: see SPEC.md for the versioned field encoding.
#[must_use]
pub fn fingerprint_bytes(
    invariant: Invariant,
    code: &str,
    pattern: &[(Kind, u64)],
    sign: i8,
    relation: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut string = |text: &str| {
        bytes.extend_from_slice(&u32::try_from(text.len()).unwrap_or(u32::MAX).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    };
    string(invariant.name());
    string(code);
    let mut sorted = pattern.to_vec();
    sorted.sort_by_key(|p| p.0);
    bytes.extend_from_slice(
        &u32::try_from(sorted.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for (kind, count) in sorted {
        bytes.push(kind as u8);
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    bytes.extend_from_slice(&sign.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(relation.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    bytes.extend_from_slice(relation.as_bytes());
    bytes
}
/// Class-level heuristic hash, distinct from the exact reduction instance key.
#[must_use]
pub fn class_fingerprint(
    invariant: Invariant,
    code: &str,
    pattern: &[(Kind, u64)],
    sign: i8,
    relation: &str,
) -> String {
    format!(
        "qlrs-fp1:{}",
        blake3::hash(&fingerprint_bytes(invariant, code, pattern, sign, relation)).to_hex()
    )
}
fn window_pattern(
    store: &TraceStore,
    alive: &AliveMask,
    start: usize,
    end: usize,
) -> Vec<(Kind, u64)> {
    let mut counts = [0_u64; 8];
    for event in start..=end {
        if store
            .spine
            .unit
            .get(event)
            .is_some_and(|u| alive.contains(*u as usize))
            && let Some(kind) = store.spine.kind.get(event)
            && let Some(count) = counts.get_mut(*kind as usize)
        {
            *count = count.saturating_add(1);
        }
    }
    [
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
    .collect()
}
fn order_pattern(
    store: &TraceStore,
    alive: &AliveMask,
    order: usize,
    end: usize,
) -> Vec<(Kind, u64)> {
    let unit = store
        .orders
        .unit
        .get(order)
        .copied()
        .unwrap_or(qlrs_schema::NONE);
    let mut counts = [0_u64; 8];
    for event in 0..=end {
        if store.spine.unit.get(event) == Some(&unit)
            && alive.contains(unit as usize)
            && let Some(kind) = store.spine.kind.get(event)
            && let Some(count) = counts.get_mut(*kind as usize)
        {
            *count = count.saturating_add(1);
        }
    }
    let kinds = [
        Kind::Submitted,
        Kind::Accepted,
        Kind::Rejected,
        Kind::Filled,
        Kind::Canceled,
        Kind::CashClaim,
        Kind::PositionClaim,
        Kind::Unsupported,
    ];
    kinds
        .into_iter()
        .zip(counts)
        .filter(|(_, c)| *c > 0)
        .collect()
}
fn relation(
    store: &TraceStore,
    alive: &AliveMask,
    start: usize,
    end: usize,
    residual: i128,
    expected: i128,
    observed: i128,
) -> String {
    let last = (start..end).rev().find(|event| {
        store.spine.kind.get(*event) == Some(&Kind::Filled)
            && store
                .spine
                .unit
                .get(*event)
                .is_some_and(|u| alive.contains(*u as usize))
    });
    if let Some(fill) = last
        .and_then(|e| store.spine.payload_idx.get(e))
        .map(|f| *f as usize)
    {
        let fee = store
            .fills
            .fee_raw
            .get(fill)
            .zip(store.fills.fee_scale.get(fill))
            .and_then(|(raw, scale)| {
                Fixed::new(*raw, *scale)
                    .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
                    .ok()
            })
            .map(qlrs_num::Fixed::raw);
        if fee.is_some_and(|f| f != 0 && residual == f) {
            return "+fee".into();
        }
        if fee.and_then(i128::checked_neg) == Some(residual) {
            return "-fee".into();
        }
        let px = store
            .fills
            .px_raw
            .get(fill)
            .zip(store.fills.px_scale.get(fill))
            .and_then(|(raw, scale)| {
                Fixed::new(*raw, *scale)
                    .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
                    .ok()
            })
            .map(qlrs_num::Fixed::raw);
        if let Some(px) = px {
            for k in 1..=10_i128 {
                if px.checked_mul(k) == Some(residual) {
                    return format!("+{k}*last_px");
                }
                if px.checked_mul(k).and_then(i128::checked_neg) == Some(residual) {
                    return format!("-{k}*last_px");
                }
            }
        }
        let notional = store
            .fills
            .qty_raw
            .get(fill)
            .zip(store.fills.qty_scale.get(fill))
            .zip(
                store
                    .fills
                    .px_raw
                    .get(fill)
                    .zip(store.fills.px_scale.get(fill)),
            )
            .and_then(|((q, qs), (p, ps))| {
                Fixed::new(*q, *qs)
                    .and_then(|q| {
                        q.mul_rescale(Fixed::new(*p, *ps)?, store.cash_scale, Rounding::Exact)
                    })
                    .ok()
            })
            .map(qlrs_num::Fixed::raw);
        if notional == Some(residual) {
            return "+notional".into();
        }
        if notional.and_then(i128::checked_neg) == Some(residual) {
            return "-notional".into();
        }
    }
    for exponent in 1..=18_u32 {
        let factor = 10_i128.checked_pow(exponent);
        if factor.and_then(|f| expected.checked_mul(f)) == Some(observed) {
            return format!("scale-error:observed=+10^{exponent}*expected");
        }
        if factor
            .and_then(|f| expected.checked_mul(f))
            .and_then(i128::checked_neg)
            == Some(observed)
        {
            return format!("scale-error:observed=-10^{exponent}*expected");
        }
    }
    "other".into()
}

fn observed_text(store: &TraceStore, id: u32, raw: i128, scale: u8) -> String {
    store.strings.get(id).map_or_else(
        || Fixed::new(raw, scale).map_or_else(|_| String::new(), |v| v.to_string()),
        str::to_owned,
    )
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests {
    use super::*;
    #[test]
    fn req_fp_001_canonical_vector_and_order_independence() {
        let pattern = [(Kind::Filled, 1), (Kind::CashClaim, 1)];
        let bytes = fingerprint_bytes(Invariant::I1, "cash-conservation", &pattern, -1, "-fee");
        let mut hex = String::new();
        for byte in &bytes {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").unwrap();
        }
        assert_eq!(
            hex,
            include_str!("../../../fixtures/golden/fingerprint-vector.hex").trim()
        );
        assert_eq!(
            class_fingerprint(Invariant::I1, "cash-conservation", &pattern, -1, "-fee"),
            "qlrs-fp1:d36cbaabf8cae916908640e54736cc3856466a66ffc4e3d37f08f9f55d373a72"
        );
        assert_eq!(
            class_fingerprint(Invariant::I1, "cash-conservation", &pattern, -1, "-fee"),
            class_fingerprint(
                Invariant::I1,
                "cash-conservation",
                &[(Kind::CashClaim, 1), (Kind::Filled, 1)],
                -1,
                "-fee"
            )
        );
        assert_eq!(Invariant::I1.name(), "I1");
        assert_eq!(Invariant::I2.name(), "I2");
        assert_eq!(Invariant::I3.name(), "I3");
    }
    #[test]
    fn req_fp_002_at_seq_filters_status_and_anchor() {
        let mut report = Report {
            status: Status::Inconclusive,
            findings: vec![limitation(Status::Inconclusive, 7, "missing")],
            expected_cash: String::new(),
            expected_positions: Vec::new(),
        };
        assert!(report.at_seq(7).is_none());
        let mut finding = report.findings.first().unwrap().clone();
        finding.status = Status::Violation;
        report.findings.push(finding);
        assert!(report.at_seq(7).is_some());
        assert!(report.at_seq(8).is_none());
    }
}
