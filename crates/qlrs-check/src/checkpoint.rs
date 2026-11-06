//! Claim-boundary context derived from original expected state and observations.
use crate::{
    Checker,
    lifecycle::{ProgressTable, quantity_limit},
};
use qlrs_ledger::Ledger;
use qlrs_num::{Fixed, Rounding};
use qlrs_schema::{
    CheckpointCashWindow, CheckpointOrder, CheckpointPosition, CheckpointPositionWindow,
    CheckpointTrade, Kind, OrderState, Side, SyntheticCheckpoint, TraceStore,
};
use std::collections::{BTreeSet, HashSet};

fn observed(store: &TraceStore, symbol: u32, raw: i128, scale: u8) -> Result<String, String> {
    store.strings.get(symbol).map_or_else(
        || {
            Fixed::new(raw, scale)
                .map(|v| v.to_string())
                .map_err(|e| e.to_string())
        },
        |text| Ok(text.to_owned()),
    )
}
fn raw(text: &str, scale: u8) -> Result<i128, String> {
    Fixed::parse(text, 18)
        .and_then(|v| v.rescale(scale, Rounding::Exact))
        .map(qlrs_num::Fixed::raw)
        .map_err(|e| e.to_string())
}
fn decimal(value: i128, scale: u8) -> Result<String, String> {
    Fixed::new(value, scale)
        .map(|v| v.to_string())
        .map_err(|e| e.to_string())
}
impl Checker {
    /// Compute expected state, lifecycle progress and observed window context at
    /// a claim boundary. Only orders/trades referenced by the suffix are kept.
    // SAFETY-BOUNDS: prepared I/O columns have validated shapes and references;
    // all source money arithmetic still goes through checked ledger operations.
    #[allow(clippy::indexing_slicing, clippy::too_many_lines)]
    pub fn checkpoint_at_claim(
        store: &TraceStore,
        cut: u64,
        end: u64,
        input: Option<&SyntheticCheckpoint>,
        digest: &str,
    ) -> Result<SyntheticCheckpoint, String> {
        let Ok(boundary) = store.spine.seq.binary_search(&cut) else {
            return input
                .filter(|cp| cp.cut_seq == cut)
                .cloned()
                .ok_or_else(|| "missing checkpoint boundary".into());
        };
        if !matches!(
            store.spine.kind[boundary],
            Kind::CashClaim | Kind::PositionClaim
        ) && input.is_none_or(|cp| cp.cut_seq != cut)
        {
            return Err("checkpoint boundary must be an observed claim".into());
        }
        if input.is_some_and(|cp| cp.cut_seq > cut) {
            return Err("checkpoint cannot move backwards".into());
        }
        let mut relevant_orders = BTreeSet::new();
        let mut relevant_trades = HashSet::new();
        let stop = store.spine.seq.partition_point(|seq| *seq <= end);
        for event in boundary..stop {
            let unit = store.spine.unit[event] as usize;
            if let Some(order) = store.units.get(unit).and_then(|unit| unit.order) {
                relevant_orders.insert(order as usize);
            }
            if store.spine.kind[event] == Kind::Filled {
                let fill = store.spine.payload_idx[event] as usize;
                let order = store.fills.order[fill] as usize;
                relevant_trades.insert((
                    store.orders.instrument[order] as usize,
                    store.fills.trade[fill],
                ));
            }
        }
        let mut state = Ledger::initial(store, input)?;
        let mut progress = ProgressTable::new(store, input, true)?;
        let mut cash =
            input
                .and_then(|cp| cp.cash_window.clone())
                .unwrap_or(CheckpointCashWindow {
                    seq: None,
                    observed: decimal(state.cash, store.cash_scale)?,
                    delta: decimal(0, store.cash_scale)?,
                });
        let mut cash_delta = raw(&cash.delta, store.cash_scale)?;
        let mut windows = Vec::new();
        let mut deltas = Vec::new();
        for (index, metadata) in store.header.instruments.iter().enumerate() {
            let window = input
                .and_then(|cp| {
                    cp.position_windows
                        .iter()
                        .find(|window| window.instrument_id == metadata.id)
                })
                .cloned()
                .unwrap_or(CheckpointPositionWindow {
                    instrument_id: metadata.id.clone(),
                    seq: None,
                    observed: decimal(state.positions[index], metadata.qty_precision)?,
                    delta: decimal(0, metadata.qty_precision)?,
                });
            deltas.push(raw(&window.delta, metadata.qty_precision)?);
            windows.push(window);
        }
        let mut trades = BTreeSet::new();
        if let Some(cp) = input {
            for trade in &cp.trades {
                let instrument = store
                    .header
                    .instruments
                    .iter()
                    .position(|i| i.id == trade.instrument_id)
                    .ok_or("unknown checkpoint trade instrument")?;
                if store
                    .strings
                    .lookup(&trade.trade_id)
                    .is_some_and(|symbol| relevant_trades.contains(&(instrument, symbol)))
                {
                    trades.insert((trade.instrument_id.clone(), trade.trade_id.clone()));
                }
            }
        }
        let start = input.map_or(0, |cp| {
            store.spine.seq.partition_point(|seq| *seq < cp.cut_seq)
        });
        for event in start..=boundary {
            let kind = store.spine.kind[event];
            let payload = store.spine.payload_idx[event] as usize;
            let seq = store.spine.seq[event];
            if kind == Kind::CashClaim {
                cash.seq = Some(seq);
                cash.observed = observed(
                    store,
                    store.cash_claims.text[payload],
                    store.cash_claims.raw[payload],
                    store.cash_claims.scale[payload],
                )?;
                cash_delta = 0;
            }
            if kind == Kind::PositionClaim {
                let instrument = store.position_claims.instrument[payload] as usize;
                windows[instrument].seq = Some(seq);
                windows[instrument].observed = observed(
                    store,
                    store.position_claims.text[payload],
                    store.position_claims.raw[payload],
                    store.position_claims.scale[payload],
                )?;
                deltas[instrument] = 0;
            }
            if event == boundary {
                break;
            }
            if kind == Kind::Filled {
                state
                    .apply(store, event)
                    .map_err(|e| format!("checkpoint {} at {}", e.reason, e.seq))?;
                cash_delta = cash_delta
                    .checked_add(store.delta_cash[payload])
                    .ok_or("checkpoint cash overflow")?;
                let order = store.fills.order[payload] as usize;
                let instrument = store.orders.instrument[order] as usize;
                let qty = store.fill_qty[payload];
                let delta = if store.orders.side[order] == Side::Buy {
                    Some(qty)
                } else {
                    qty.checked_neg()
                }
                .ok_or("checkpoint position overflow")?;
                deltas[instrument] = deltas[instrument]
                    .checked_add(delta)
                    .ok_or("checkpoint position overflow")?;
                if relevant_trades.contains(&(instrument, store.fills.trade[payload])) {
                    trades.insert((
                        store.header.instruments[instrument].id.clone(),
                        store
                            .strings
                            .get(store.fills.trade[payload])
                            .ok_or("missing trade symbol")?
                            .to_owned(),
                    ));
                }
            }
            if matches!(
                kind,
                Kind::Submitted | Kind::Accepted | Kind::Rejected | Kind::Filled | Kind::Canceled
            ) {
                let order = if kind == Kind::Filled {
                    store.fills.order[payload] as usize
                } else {
                    payload
                };
                if relevant_orders.contains(&order) {
                    let qty = if kind == Kind::Filled {
                        store.fill_qty[payload]
                    } else {
                        0
                    };
                    progress
                        .get_mut(order)
                        .ok_or("invalid order")?
                        .observe(kind, seq, qty, quantity_limit(store, order).ok())
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        let mut orders = Vec::new();
        for order in relevant_orders {
            let progress = progress.get_mut(order).ok_or("invalid order state")?;
            if progress.state == OrderState::Missing {
                continue;
            }
            let metadata = store
                .header
                .instruments
                .get(store.orders.instrument[order] as usize)
                .ok_or("unknown order instrument")?;
            orders.push(CheckpointOrder {
                client_order_id: store
                    .strings
                    .get(store.orders.id[order])
                    .ok_or("missing order name")?
                    .to_owned(),
                instrument_id: metadata.id.clone(),
                side: store.orders.side[order],
                quantity: decimal(
                    quantity_limit(store, order).map_err(|e| e.to_string())?,
                    metadata.qty_precision,
                )?,
                status: progress.state,
                filled_quantity: decimal(progress.total, metadata.qty_precision)?,
            });
        }
        orders.sort_by(|left, right| left.client_order_id.cmp(&right.client_order_id));
        cash.delta = decimal(cash_delta, store.cash_scale)?;
        for (window, delta) in windows.iter_mut().zip(deltas) {
            let metadata = store
                .header
                .instruments
                .iter()
                .find(|i| i.id == window.instrument_id)
                .ok_or("unknown position")?;
            window.delta = decimal(delta, metadata.qty_precision)?;
        }
        let source_digest = input
            .filter(|cp| cp.cut_seq == cut)
            .map_or(digest, |cp| cp.source_digest.as_str())
            .to_owned();
        Ok(SyntheticCheckpoint {
            schema_version: 1,
            provenance: "synthetic".into(),
            account_id: store.account().into(),
            currency: store.header.model.base_currency.clone(),
            cash: decimal(state.cash, store.cash_scale)?,
            positions: store
                .header
                .instruments
                .iter()
                .zip(&state.positions)
                .map(|(i, raw)| {
                    Ok(CheckpointPosition {
                        instrument_id: i.id.clone(),
                        net_qty: decimal(*raw, i.qty_precision)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            cut_seq: cut,
            description: format!("computed by qlrs ledger from original trace {source_digest}"),
            source_digest,
            orders,
            trades: trades
                .into_iter()
                .map(|(instrument_id, trade_id)| CheckpointTrade {
                    instrument_id,
                    trade_id,
                })
                .collect(),
            cash_window: Some(cash),
            position_windows: windows,
        })
    }
}
