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
mod checkpoint;
mod fingerprint;
mod lifecycle;
pub use fingerprint::{class_fingerprint, fingerprint_bytes};
use lifecycle::{ProgressTable, terminal};
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
    /// Restore checkpoint order metadata before preparing a truncated witness.
    pub fn prepare_with_checkpoint(
        store: &mut TraceStore,
        checkpoint: Option<&SyntheticCheckpoint>,
    ) -> Result<(), String> {
        Ledger::prepare_with_checkpoint(store, checkpoint)
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
        Self::check_events(store, alive, checkpoint, partial, None, 0..store.len())
    }
    /// Complete invariant checking over sorted retained event indices.
    /// The same core checker is used for dense and sparse replay.
    #[must_use]
    pub fn check_selected(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
        partial: bool,
        events: &[usize],
    ) -> Report {
        Self::check_events(
            store,
            alive,
            checkpoint,
            partial,
            Some(events),
            events.iter().copied(),
        )
    }
    /// Replay a witness under its declared target scope. Tail-coverage checks
    /// belong to full traces; all retained lifecycle/model checks still apply.
    #[must_use]
    pub fn check_witness(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
        target: &InstanceKey,
    ) -> Report {
        witness_report(
            Self::check(store, alive, checkpoint, true),
            target,
            store,
            alive,
        )
    }
    /// Scoped witness replay over sorted retained event indices.
    #[must_use]
    pub fn check_witness_selected(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
        target: &InstanceKey,
        events: &[usize],
    ) -> Report {
        witness_report(
            Self::check_selected(store, alive, checkpoint, true, events),
            target,
            store,
            alive,
        )
    }
    // SAFETY-BOUNDS: validated column shapes and payload references guard the core loop.
    #[allow(clippy::indexing_slicing)]
    fn check_events(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
        partial: bool,
        events: Option<&[usize]>,
        indices: impl Iterator<Item = usize>,
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
        if events.is_some_and(|events| {
            events.last().is_some_and(|event| *event >= store.len())
                || events.windows(2).any(|pair| pair.first() >= pair.get(1))
        }) {
            report.findings.push(limitation(
                Status::Inconclusive,
                0,
                "invalid-selected-event-order",
            ));
            return finish(report, partial);
        }
        let applicable: Vec<_> = store
            .issues
            .iter()
            .filter(|issue| {
                issue.seq.is_none_or(|seq| {
                    checkpoint.is_none_or(|cp| seq >= cp.cut_seq)
                        && store
                            .spine
                            .seq
                            .binary_search(&seq)
                            .is_ok_and(|event| alive.contains_event(store, event))
                })
            })
            .collect();
        for issue in &applicable {
            report.findings.push(limitation(
                issue.status,
                issue.seq.unwrap_or(0),
                &issue.reason,
            ));
        }
        if applicable
            .iter()
            .any(|issue| issue.seq.is_none() || (issue.status == Status::Unsupported && !partial))
        {
            return finish(report, partial);
        }
        let uncertain_after = applicable.iter().filter_map(|issue| issue.seq).min();
        let mut state = match Ledger::initial(store, checkpoint) {
            Ok(s) => s,
            Err(reason) => {
                report
                    .findings
                    .push(limitation(Status::Inconclusive, 0, &reason));
                return finish(report, partial);
            }
        };
        let mut progress = match ProgressTable::new(store, checkpoint, events.is_some()) {
            Ok(value) => value,
            Err(reason) => {
                report
                    .findings
                    .push(limitation(Status::Inconclusive, 0, &reason));
                return finish(report, partial);
            }
        };
        let mut trades = HashSet::with_capacity_and_hasher(
            store
                .fills
                .order
                .len()
                .min(events.map_or(usize::MAX, <[usize]>::len)),
            foldhash::fast::RandomState::default(),
        );
        if let Some(cp) = checkpoint {
            for trade in &cp.trades {
                let Some(instrument) = store
                    .header
                    .instruments
                    .iter()
                    .position(|i| i.id == trade.instrument_id)
                    .and_then(|index| u32::try_from(index).ok())
                else {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        0,
                        "unknown-checkpoint-trade-instrument",
                    ));
                    return finish(report, partial);
                };
                if let Some(symbol) = store.strings.lookup(&trade.trade_id) {
                    trades.insert((instrument, symbol));
                }
            }
        }
        let mut base = state.cash;
        let mut window_delta = 0_i128;
        let mut previous = None;
        let mut window_start = checkpoint.map_or(0, |cp| {
            store.spine.seq.partition_point(|seq| *seq < cp.cut_seq)
        });
        let mut cash_pattern = [0_u64; 8];
        let mut prefix_pattern = [0_u64; 8];
        let mut position_pattern = vec![[0_u64; 8]; state.positions.len()];
        let mut position_base = state.positions.clone();
        let mut position_delta = vec![0_i128; state.positions.len()];
        let mut previous_position = vec![None; state.positions.len()];
        let mut pending_cash = None;
        let mut pending_positions = vec![None; state.positions.len()];
        if let Some(cp) = checkpoint {
            if let Some(opening) = &cp.cash_window {
                let context = Fixed::parse(&opening.observed, 18)
                    .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
                    .and_then(|v| {
                        Ok((
                            v.raw(),
                            Fixed::parse(&opening.delta, 18)?
                                .rescale(store.cash_scale, Rounding::Exact)?
                                .raw(),
                        ))
                    });
                match context {
                    Ok((value, delta)) if opening.seq.is_none_or(|seq| seq <= cp.cut_seq) => {
                        base = value;
                        window_delta = delta;
                        previous = opening.seq;
                    }
                    _ => {
                        report.findings.push(limitation(
                            Status::Inconclusive,
                            cp.cut_seq,
                            "invalid-opening-cash",
                        ));
                        return finish(report, partial);
                    }
                }
            }
            let mut seen = HashSet::new();
            for opening in &cp.position_windows {
                let Some(index) = store
                    .header
                    .instruments
                    .iter()
                    .position(|i| i.id == opening.instrument_id)
                else {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        cp.cut_seq,
                        "invalid-opening-position",
                    ));
                    return finish(report, partial);
                };
                let scale = store.header.instruments[index].qty_precision;
                let value = Fixed::parse(&opening.observed, scale)
                    .and_then(|v| v.rescale(scale, Rounding::Exact));
                let delta = Fixed::parse(&opening.delta, scale)
                    .and_then(|v| v.rescale(scale, Rounding::Exact));
                match (value, delta) {
                    (Ok(value), Ok(delta))
                        if seen.insert(index)
                            && opening.seq.is_none_or(|seq| seq <= cp.cut_seq) =>
                    {
                        position_base[index] = value.raw();
                        position_delta[index] = delta.raw();
                        previous_position[index] = opening.seq;
                    }
                    _ => {
                        report.findings.push(limitation(
                            Status::Inconclusive,
                            cp.cut_seq,
                            "invalid-opening-position",
                        ));
                        return finish(report, partial);
                    }
                }
            }
        }
        let mut financial = true;
        let start = checkpoint.map_or(0, |cp| {
            store.spine.seq.partition_point(|seq| *seq < cp.cut_seq)
        });
        for event in indices {
            if !alive.contains_event(store, event) || event < start {
                continue;
            }
            let kind = store.spine.kind[event];
            let payload = store.spine.payload_idx[event] as usize;
            let seq = store.spine.seq[event];
            if uncertain_after.is_some_and(|cut| seq >= cut) {
                break;
            }
            if let Some(count) = cash_pattern.get_mut(kind as usize) {
                *count = count.saturating_add(1);
            }
            if let Some(count) = prefix_pattern.get_mut(kind as usize) {
                *count = count.saturating_add(1);
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
                if order >= store.orders.id.len() {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        seq,
                        "invalid-order-index",
                    ));
                    financial = false;
                    continue;
                }
                let Some(order_state) = progress.get_mut(order) else {
                    report.findings.push(limitation(
                        Status::Inconclusive,
                        seq,
                        "invalid-order-index",
                    ));
                    financial = false;
                    continue;
                };
                let limit = store.orders.quantity_limit.get(order).copied().flatten();
                let qty = if kind == Kind::Filled {
                    store.fill_qty.get(payload).copied().unwrap_or(0)
                } else {
                    0
                };
                let Ok(state_before) = order_state.observe(kind, seq, qty, limit) else {
                    report
                        .findings
                        .push(limitation(Status::Inconclusive, seq, "numeric-overflow"));
                    financial = false;
                    continue;
                };
                let mut codes = Vec::new();
                let terminal = terminal(state_before);
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
                    if pending_cash.is_none() {
                        pending_cash = Some(seq);
                    }
                    if let Some(pending) = pending_positions.get_mut(instrument)
                        && pending.is_none()
                    {
                        *pending = Some(seq);
                    }
                    if store.header.instruments.get(instrument).is_some()
                        && !trades
                            .insert((store.orders.instrument[order], store.fills.trade[payload]))
                    {
                        codes.push("duplicate-trade-id");
                    }
                    if let Some(metadata) = store.header.instruments.get(instrument) {
                        match Fixed::new(store.orders.qty_raw[order], store.orders.qty_scale[order])
                            .and_then(|q| q.rescale(metadata.qty_precision, Rounding::Exact))
                        {
                            Ok(quantity) => {
                                if order_state.total > quantity.raw() {
                                    codes.push("overfill");
                                    excess =
                                        order_state.total.checked_sub(quantity.raw()).unwrap_or(0);
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
                    let pattern = counts_pattern(&order_state.counts);
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
                            start_seq: order_state.first,
                            end_seq: seq,
                        },
                        &pattern,
                        "other",
                    ));
                }
            }
            if kind == Kind::Filled && financial {
                if let Err(error) = state.apply(store, event) {
                    report
                        .findings
                        .push(limitation(error.status, seq, error.reason));
                    financial = false;
                }
                let order = store.fills.order[payload] as usize;
                let instrument = store.orders.instrument[order] as usize;
                if let Some(delta) = position_delta.get_mut(instrument) {
                    let quantity = store.fill_qty[payload];
                    let signed = if store.orders.side[order] == qlrs_schema::Side::Buy {
                        Some(quantity)
                    } else {
                        quantity.checked_neg()
                    };
                    if let Some(sum) = signed.and_then(|quantity| delta.checked_add(quantity)) {
                        *delta = sum;
                    } else {
                        report.findings.push(limitation(
                            Status::Inconclusive,
                            seq,
                            "numeric-overflow",
                        ));
                        financial = false;
                    }
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
                pending_cash = None;
                let opening = checkpoint.is_some_and(|cp| {
                    seq == cp.cut_seq
                        || cp.cash_window.as_ref().and_then(|window| window.seq) == Some(seq)
                });
                if opening
                    && let Ok(value) = Fixed::new(
                        store.cash_claims.raw[payload],
                        store.cash_claims.scale[payload],
                    )
                    .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
                {
                    base = value.raw();
                }
                if financial && !opening {
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
                                    let pattern = counts_pattern(&cash_pattern);
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
                cash_pattern.fill(0);
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
                pending_positions[instrument] = None;
                let opening = checkpoint.is_some_and(|cp| {
                    seq == cp.cut_seq
                        || cp.position_windows.iter().any(|window| {
                            window.instrument_id == store.header.instruments[instrument].id
                                && window.seq == Some(seq)
                        })
                });
                if financial && !opening {
                    let metadata = &store.header.instruments[instrument];
                    let scale = metadata.qty_precision;
                    match Fixed::new(
                        store.position_claims.raw[payload],
                        store.position_claims.scale[payload],
                    )
                    .and_then(|v| v.rescale(scale, Rounding::Exact))
                    {
                        Ok(observed) => {
                            if let Some(expected) =
                                position_base[instrument].checked_add(position_delta[instrument])
                                && let Some(residual) = observed.raw().checked_sub(expected)
                            {
                                if residual != 0 && !opening {
                                    if report.findings.is_empty() {
                                        report.findings.reserve(store.position_claims.raw.len());
                                    }
                                    report.findings.push(violation(
                                        store,
                                        Invariant::I2,
                                        "position-sum",
                                        &metadata.id,
                                        seq,
                                        residual,
                                        scale,
                                        Fixed::new(expected, scale).ok().map(|v| v.to_string()),
                                        Some(observed_text(
                                            store,
                                            store.position_claims.text[payload],
                                            store.position_claims.raw[payload],
                                            store.position_claims.scale[payload],
                                        )),
                                        Window {
                                            start_seq: previous_position[instrument],
                                            end_seq: seq,
                                        },
                                        &counts_pattern(&pattern_difference(
                                            &prefix_pattern,
                                            &position_pattern[instrument],
                                        )),
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
                if let Ok(observed) = Fixed::new(
                    store.position_claims.raw[payload],
                    store.position_claims.scale[payload],
                )
                .and_then(|v| {
                    v.rescale(
                        store.header.instruments[instrument].qty_precision,
                        Rounding::Exact,
                    )
                }) {
                    position_base[instrument] = observed.raw();
                }
                position_delta[instrument] = 0;
                previous_position[instrument] = Some(seq);
                position_pattern[instrument] = prefix_pattern;
            }
        }
        if let Some(seq) = pending_cash {
            report.findings.push(limitation(
                Status::Inconclusive,
                seq,
                "missing-later-account-claim",
            ));
        }
        for seq in pending_positions.into_iter().flatten() {
            report.findings.push(limitation(
                Status::Inconclusive,
                seq,
                "missing-later-position-claim",
            ));
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

fn witness_report(
    mut report: Report,
    target: &InstanceKey,
    store: &TraceStore,
    alive: &AliveMask,
) -> Report {
    let missing_opening = report
        .findings
        .iter()
        .filter(|finding| finding.instance_key.as_ref() == Some(target))
        .filter(|_| target.invariant_id != Invariant::I3)
        .filter_map(|finding| finding.window.start_seq)
        .any(|seq| {
            store
                .spine
                .seq
                .binary_search(&seq)
                .ok()
                .is_none_or(|event| !alive.contains_event(store, event))
        });
    if missing_opening {
        report.findings.push(limitation(
            Status::Inconclusive,
            target.anchor_claim_original_seq,
            "missing-witness-anchor",
        ));
    }
    report.findings.retain(|finding| {
        !finding
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("missing-later-"))
            && finding.instance_key.as_ref().is_none_or(|key| {
                key.invariant_id == Invariant::I3
                    || (key.invariant_id == target.invariant_id
                        && key.account == target.account
                        && key.scope == target.scope)
            })
    });
    finish(report, true)
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
fn counts_pattern<const N: usize>(counts: &[u64; N]) -> Vec<(Kind, u64)> {
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
    .zip(*counts)
    .filter(|(_, count)| *count > 0)
    .collect()
}
fn pattern_difference(current: &[u64; 8], previous: &[u64; 8]) -> [u64; 8] {
    std::array::from_fn(|index| {
        current
            .get(index)
            .copied()
            .unwrap_or(0)
            .saturating_sub(previous.get(index).copied().unwrap_or(0))
    })
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
        store.spine.kind.get(*event) == Some(&Kind::Filled) && alive.contains_event(store, *event)
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
