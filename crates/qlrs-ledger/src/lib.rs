//! Independent, checked replay over precomputed columnar fill deltas.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic, clippy::arithmetic_side_effects)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]
use qlrs_num::{Fixed, NumericError, Rounding};
use qlrs_schema::{
    AliveMask, CashRounding, Issue, Kind, Side, Status, SyntheticCheckpoint, TraceStore,
};

/// Independently computed state. Creating it allocates; applying fills does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedState {
    /// Account cash at `TraceStore::cash_scale`.
    pub cash: i128,
    /// Positions indexed by dense instrument index, at each declared quantity scale.
    pub positions: Vec<i128>,
}
/// Replay failure, with no heap allocation in the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayError {
    /// Source sequence at failure.
    pub seq: u64,
    /// Non-accusatory classification.
    pub status: Status,
    /// Stable machine-readable reason.
    pub reason: &'static str,
}
/// Columnar preparation and independent cash/position replay.
pub struct Ledger;
impl Ledger {
    /// Compute checked deltas once. Idempotent; does not use observed balances.
    pub fn prepare(store: &mut TraceStore) {
        if store.prepared {
            return;
        }
        store.prepared = true;
        store.delta_cash.resize(store.len(), 0);
        store.fill_qty.resize(store.fills.order.len(), 0);
        let Some(currency) = store.header.currencies.first() else {
            return;
        };
        let precision = currency.precision.min(18);
        store.cash_scale = store
            .header
            .model
            .commission_max_scale
            .unwrap_or(precision)
            .max(precision)
            .min(18);
        if store.issues.iter().any(|issue| issue.seq.is_none()) {
            return;
        }
        let initial = store
            .header
            .accounts
            .first()
            .and_then(|a| a.starting_balance.parse::<Fixed>().ok())
            .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact).ok());
        let Some(initial) = initial else {
            store.issues.push(Issue {
                status: Status::Inconclusive,
                seq: None,
                reason: "numeric-overflow".into(),
            });
            return;
        };
        store.starting_cash = initial.raw();
        if initial.raw() < 0 {
            store.issues.push(Issue {
                status: Status::Unsupported,
                seq: None,
                reason: "negative-starting-cash".into(),
            });
        }
        for event in 0..store.len() {
            if store.spine.kind.get(event) != Some(&Kind::Filled) {
                continue;
            }
            match Self::prepare_fill(store, event) {
                Ok(()) => {}
                Err((status, reason)) => store.issues.push(Issue {
                    status,
                    seq: store.spine.seq.get(event).copied(),
                    reason: reason.into(),
                }),
            }
        }
    }
    // SAFETY-BOUNDS: the I/O builder constructs equal columns; the shape check
    // precedes replay. Filled payloads and order indices are checked below.
    #[allow(clippy::indexing_slicing)]
    fn prepare_fill(store: &mut TraceStore, event: usize) -> Result<(), (Status, &'static str)> {
        let fill = *store
            .spine
            .payload_idx
            .get(event)
            .ok_or((Status::Inconclusive, "invalid-store"))? as usize;
        let order = *store
            .fills
            .order
            .get(fill)
            .ok_or((Status::Inconclusive, "invalid-store"))? as usize;
        let Some(instrument) = store
            .orders
            .instrument
            .get(order)
            .and_then(|id| store.header.instruments.get(*id as usize))
        else {
            return Ok(());
        };
        if store.fills.qty_scale[fill] > instrument.qty_precision
            || store.fills.px_scale[fill] > instrument.px_precision
        {
            return Err((Status::Unsupported, "unsupported-scale"));
        }
        let qty = Fixed::new(store.fills.qty_raw[fill], store.fills.qty_scale[fill])
            .map_err(numeric_issue)?;
        let px = Fixed::new(store.fills.px_raw[fill], store.fills.px_scale[fill])
            .map_err(numeric_issue)?;
        let fee = Fixed::new(store.fills.fee_raw[fill], store.fills.fee_scale[fill])
            .map_err(numeric_issue)?;
        let signed_px = match store.orders.side[order] {
            Side::Buy => Fixed::new(
                px.raw()
                    .checked_neg()
                    .ok_or((Status::Inconclusive, "numeric-overflow"))?,
                px.scale(),
            )
            .map_err(numeric_issue)?,
            Side::Sell => px,
        };
        let negative_fee = Fixed::new(
            fee.raw()
                .checked_neg()
                .ok_or((Status::Inconclusive, "numeric-overflow"))?,
            fee.scale(),
        )
        .map_err(numeric_issue)?;
        let target_scale = if store.header.model.engine_cash_rounding == CashRounding::Exact {
            store.cash_scale
        } else {
            store.header.currencies.first().map_or(0, |c| c.precision)
        };
        let delta = qty
            .mul_add_rescale(
                signed_px,
                negative_fee,
                target_scale,
                store.header.model.engine_cash_rounding.numeric(),
            )
            .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
            .map_err(numeric_issue)?
            .raw();
        store.delta_cash[event] = delta;
        store.fill_qty[fill] = qty
            .rescale(instrument.qty_precision, Rounding::Exact)
            .map_err(numeric_issue)?
            .raw();
        Ok(())
    }
    /// Construct initial expected state from the header or a labelled checkpoint.
    pub fn initial(
        store: &TraceStore,
        checkpoint: Option<&SyntheticCheckpoint>,
    ) -> Result<ExpectedState, String> {
        if !store.prepared {
            return Err("unprepared-store".into());
        }
        let mut state = ExpectedState {
            cash: store.starting_cash,
            positions: vec![0; store.header.instruments.len()],
        };
        if let Some(cp) = checkpoint {
            if cp.provenance != "synthetic"
                || cp.account_id != store.account()
                || cp.currency != store.header.model.base_currency
                || cp.source_digest.len() != 64
                || !cp.source_digest.bytes().all(|b| b.is_ascii_hexdigit())
                || cp.description
                    != format!(
                        "computed by qlrs ledger from original trace {}",
                        cp.source_digest
                    )
            {
                return Err("invalid synthetic checkpoint provenance".into());
            }
            state.cash = Fixed::parse(&cp.cash, 18)
                .and_then(|v| v.rescale(store.cash_scale, Rounding::Exact))
                .map_err(|e| e.to_string())?
                .raw();
            let mut seen = vec![false; state.positions.len()];
            for position in &cp.positions {
                let index = store
                    .header
                    .instruments
                    .iter()
                    .position(|i| i.id == position.instrument_id)
                    .ok_or("unknown checkpoint instrument")?;
                let metadata = store
                    .header
                    .instruments
                    .get(index)
                    .ok_or("unknown checkpoint instrument")?;
                let flag = seen.get_mut(index).ok_or("invalid checkpoint position")?;
                if *flag {
                    return Err("duplicate checkpoint instrument".into());
                }
                *flag = true;
                let raw = Fixed::parse(&position.net_qty, metadata.qty_precision)
                    .and_then(|v| v.rescale(metadata.qty_precision, Rounding::Exact))
                    .map_err(|e| e.to_string())?
                    .raw();
                *state
                    .positions
                    .get_mut(index)
                    .ok_or("invalid checkpoint position")? = raw;
            }
            if seen.iter().any(|v| !*v) {
                return Err("checkpoint must include every instrument".into());
            }
        }
        if state.cash < 0 || state.positions.iter().any(|v| *v < 0) {
            return Err("checkpoint/header outside cash long-only model".into());
        }
        Ok(state)
    }
    /// Replay using unit-alive flags, allocating the state only before the loop.
    pub fn replay(
        store: &TraceStore,
        alive: &AliveMask,
        checkpoint: Option<&SyntheticCheckpoint>,
    ) -> Result<ExpectedState, String> {
        if let Some(issue) = store.issues.first() {
            return Err(format!("model or numeric issue: {}", issue.reason));
        }
        let mut state = Self::initial(store, checkpoint)?;
        Self::replay_into(store, alive, &mut state)
            .map_err(|e| format!("{} at seq {}", e.reason, e.seq))?;
        Ok(state)
    }
    /// Allocation-free replay loop into preallocated state.
    pub fn replay_into(
        store: &TraceStore,
        alive: &AliveMask,
        state: &mut ExpectedState,
    ) -> Result<(), ReplayError> {
        for event in 0..store.len() {
            if store
                .spine
                .unit
                .get(event)
                .is_some_and(|id| alive.contains(*id as usize))
            {
                state.apply(store, event)?;
            }
        }
        Ok(())
    }
}
impl ExpectedState {
    /// Apply a single original event, checking arithmetic and model preconditions.
    pub fn apply(&mut self, store: &TraceStore, event: usize) -> Result<(), ReplayError> {
        let seq = store.spine.seq.get(event).copied().unwrap_or(0);
        let fail = |status, reason| ReplayError {
            seq,
            status,
            reason,
        };
        if store.spine.kind.get(event) != Some(&Kind::Filled) {
            return Ok(());
        }
        let fill = *store
            .spine
            .payload_idx
            .get(event)
            .ok_or_else(|| fail(Status::Inconclusive, "invalid-store"))?
            as usize;
        let order = *store
            .fills
            .order
            .get(fill)
            .ok_or_else(|| fail(Status::Inconclusive, "invalid-store"))?
            as usize;
        let instrument = *store
            .orders
            .instrument
            .get(order)
            .ok_or_else(|| fail(Status::Inconclusive, "invalid-store"))?
            as usize;
        // Lifecycle evidence does not supply a missing side/instrument for money.
        if instrument == qlrs_schema::NONE as usize {
            return Err(fail(Status::Inconclusive, "missing-order-metadata"));
        }
        let quantity = *store
            .fill_qty
            .get(fill)
            .ok_or_else(|| fail(Status::Inconclusive, "unprepared-store"))?;
        let position = self
            .positions
            .get_mut(instrument)
            .ok_or_else(|| fail(Status::Inconclusive, "invalid-store"))?;
        *position = match store.orders.side.get(order) {
            Some(Side::Buy) => position.checked_add(quantity),
            Some(Side::Sell) => position.checked_sub(quantity),
            None => None,
        }
        .ok_or_else(|| fail(Status::Inconclusive, "numeric-overflow"))?;
        self.cash = self
            .cash
            .checked_add(
                *store
                    .delta_cash
                    .get(event)
                    .ok_or_else(|| fail(Status::Inconclusive, "unprepared-store"))?,
            )
            .ok_or_else(|| fail(Status::Inconclusive, "numeric-overflow"))?;
        if *position < 0 {
            return Err(fail(Status::Unsupported, "shorting"));
        }
        if self.cash < 0 {
            return Err(fail(Status::Unsupported, "overspend"));
        }
        Ok(())
    }
}
fn numeric_issue(error: NumericError) -> (Status, &'static str) {
    match error {
        NumericError::UnsupportedScale => (Status::Unsupported, "unsupported-scale"),
        NumericError::RoundingRequired => (Status::Inconclusive, "rounding-policy-required"),
        _ => (Status::Inconclusive, "numeric-overflow"),
    }
}
