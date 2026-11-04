//! Shared lifecycle state for checking and checkpoint construction.
use crate::TRANSITIONS;
use qlrs_num::{Fixed, NumericError, Rounding};
use qlrs_schema::{Kind, OrderState, SyntheticCheckpoint, TraceStore};
use std::collections::HashMap;

#[derive(Clone)]
pub(crate) struct Progress {
    pub state: OrderState,
    pub total: i128,
    pub first: Option<u64>,
    pub counts: [u64; 8],
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            state: OrderState::Missing,
            total: 0,
            first: None,
            counts: [0; 8],
        }
    }
}
impl Progress {
    pub fn observe(
        &mut self,
        kind: Kind,
        seq: u64,
        quantity: i128,
        limit: Option<i128>,
    ) -> Result<OrderState, NumericError> {
        let before = self.state;
        if self.first.is_none() {
            self.first = Some(seq);
        }
        if let Some(count) = self.counts.get_mut(kind as usize) {
            *count = count.saturating_add(1);
        }
        if kind == Kind::Filled {
            self.total = self
                .total
                .checked_add(quantity)
                .ok_or(NumericError::Overflow)?;
        }
        if let Some((_, _, next)) = TRANSITIONS
            .iter()
            .find(|(state, tag, _)| *state == before && *tag == kind)
        {
            self.state = *next;
        }
        if kind == Kind::Filled && !terminal(before) {
            self.state = if limit.is_some_and(|n| self.total >= n) {
                OrderState::Filled
            } else {
                OrderState::Partial
            };
        }
        Ok(before)
    }
}
pub(crate) fn terminal(state: OrderState) -> bool {
    matches!(
        state,
        OrderState::Filled | OrderState::Canceled | OrderState::Rejected
    )
}
pub(crate) fn quantity_limit(store: &TraceStore, order: usize) -> Result<i128, NumericError> {
    store
        .orders
        .quantity_limit
        .get(order)
        .copied()
        .flatten()
        .ok_or(NumericError::InvalidDecimal)
}
pub(crate) enum ProgressTable {
    Dense(Vec<Progress>),
    Sparse(HashMap<usize, Progress, foldhash::fast::RandomState>),
}
impl ProgressTable {
    pub fn new(
        store: &TraceStore,
        cp: Option<&SyntheticCheckpoint>,
        sparse: bool,
    ) -> Result<Self, String> {
        let mut table = if sparse {
            Self::Sparse(HashMap::default())
        } else {
            Self::Dense(vec![Progress::default(); store.orders.id.len()])
        };
        let mut seen = std::collections::BTreeSet::new();
        if let Some(cp) = cp {
            for order in &cp.orders {
                if !seen.insert(&order.client_order_id) {
                    return Err("duplicate checkpoint order".into());
                }
                let metadata = store
                    .header
                    .instruments
                    .iter()
                    .find(|i| i.id == order.instrument_id)
                    .ok_or("unknown checkpoint order instrument")?;
                let quantity = Fixed::parse(&order.quantity, metadata.qty_precision)
                    .and_then(|v| v.rescale(metadata.qty_precision, Rounding::Exact))
                    .map_err(|e| e.to_string())?;
                let total = Fixed::parse(&order.filled_quantity, metadata.qty_precision)
                    .and_then(|v| v.rescale(metadata.qty_precision, Rounding::Exact))
                    .map_err(|e| e.to_string())?
                    .raw();
                if order.status == OrderState::Missing || quantity.raw() <= 0 || total < 0 {
                    return Err("invalid checkpoint order state".into());
                }
                if let Some(index) = store.order_index(&order.client_order_id) {
                    if quantity_limit(store, index).map_err(|e| e.to_string())? != quantity.raw()
                        || store.orders.side.get(index) != Some(&order.side)
                        || store
                            .orders
                            .instrument
                            .get(index)
                            .and_then(|id| store.header.instruments.get(*id as usize))
                            .map(|i| &i.id)
                            != Some(&order.instrument_id)
                    {
                        return Err("checkpoint order metadata mismatch".into());
                    }
                    let state = table
                        .get_mut(index)
                        .ok_or("invalid checkpoint order index")?;
                    state.state = order.status;
                    state.total = total;
                }
            }
        }
        Ok(table)
    }
    pub fn get_mut(&mut self, order: usize) -> Option<&mut Progress> {
        match self {
            Self::Dense(states) => states.get_mut(order),
            Self::Sparse(states) => Some(states.entry(order).or_default()),
        }
    }
}
