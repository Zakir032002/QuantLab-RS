//! Checked target-scope linear contributions. Admission still uses the Checker.
use crate::{UnitEvents, event_range};
use qlrs_check::{InstanceKey, Invariant};
use qlrs_schema::{AliveMask, CashRounding, Kind, Side, TraceStore};

/// Additive numerical queries, separate from the full acceptance predicate.
pub trait FastPredicate {
    /// Target residual for a mask; None requests a full numerical replay.
    fn residual(&self, mask: &AliveMask) -> Option<i128>;
    /// Target residual for the first `end` units in canonical order, in O(1).
    fn prefix_residual(&self, end: usize) -> Option<i128>;
}
/// Window-local contribution columns and checked prefix sums.
#[derive(Debug)]
pub struct ContributionTable {
    /// Canonical original unit IDs.
    pub units: Vec<u32>,
    /// Signed raw contribution at the target's scale, one per unit.
    pub values: Vec<i128>,
    prefix: Vec<i128>,
    constant: i128,
    linear: bool,
}
impl ContributionTable {
    /// Prepare once from a complete target window. Nonlinear rounding and I3
    /// conservatively fall back to the full checker.
    pub fn new(
        store: &TraceStore,
        window: &AliveMask,
        target: &InstanceKey,
    ) -> Result<Self, String> {
        let events = UnitEvents::new(store, window)?;
        let units = events.unit_ids;
        let mut values = vec![0_i128; units.len()];
        let mut linear = target.invariant_id == Invariant::I2
            || (target.invariant_id == Invariant::I1
                && store.header.model.engine_cash_rounding == CashRounding::Exact);
        for event in event_range(store, window) {
            if !window.contains_event(store, event)
                || store.spine.kind.get(event) != Some(&Kind::Filled)
            {
                continue;
            }
            let unit = *store.spine.unit.get(event).ok_or("invalid unit")?;
            let row = units
                .binary_search(&unit)
                .map_err(|_| "missing contribution row")?;
            let fill = *store
                .spine
                .payload_idx
                .get(event)
                .ok_or("invalid payload")? as usize;
            let order = *store.fills.order.get(fill).ok_or("invalid order")? as usize;
            let value = match target.invariant_id {
                Invariant::I1 => store.delta_cash.get(event).copied(),
                Invariant::I2 => {
                    let instrument = store
                        .orders
                        .instrument
                        .get(order)
                        .and_then(|id| store.header.instruments.get(*id as usize))
                        .ok_or("invalid instrument")?;
                    if instrument.id == target.scope {
                        store.fill_qty.get(fill).copied().and_then(|qty| {
                            if store.orders.side.get(order) == Some(&Side::Sell) {
                                qty.checked_neg()
                            } else {
                                Some(qty)
                            }
                        })
                    } else {
                        Some(0)
                    }
                }
                Invariant::I3 => None,
            };
            if let Some(sum) = value.and_then(|value| values.get(row)?.checked_add(value)) {
                *values.get_mut(row).ok_or("invalid row")? = sum;
            } else {
                linear = false;
            }
        }
        let mut prefix = vec![0_i128];
        for value in &values {
            if let Some(sum) = prefix.last().and_then(|last| last.checked_add(*value)) {
                prefix.push(sum);
            } else {
                linear = false;
                prefix.push(0);
            }
        }
        let residual = target
            .residual_raw
            .parse::<i128>()
            .map_err(|e| e.to_string())?;
        let constant = residual.checked_add(*prefix.last().ok_or("missing total")?);
        linear &= constant.is_some();
        Ok(Self {
            units,
            values,
            prefix,
            constant: constant.unwrap_or(0),
            linear,
        })
    }
    /// Units with a nonzero contribution; the graph supplies prerequisites.
    #[must_use]
    pub fn seed_roots(&self) -> Vec<usize> {
        self.units
            .iter()
            .zip(&self.values)
            .filter(|(_, value)| **value != 0)
            .map(|(unit, _)| *unit as usize)
            .collect()
    }
    /// Prefix sum of a union of closures, counting shared prerequisites once.
    /// The resulting table answers each prefix in O(1).
    #[must_use]
    pub fn closure_prefixes(&self, closures: &[Vec<usize>], base: &AliveMask) -> Option<Vec<i128>> {
        if !self.linear {
            return None;
        }
        let mut kept = base.clone();
        let mut residual = self.residual(base)?;
        let mut out = vec![residual];
        for closure in closures {
            for unit in closure {
                if kept.contains(*unit) {
                    continue;
                }
                kept.set(*unit, true);
                if let Ok(id) = u32::try_from(*unit)
                    && let Ok(row) = self.units.binary_search(&id)
                {
                    residual = residual.checked_sub(*self.values.get(row)?)?;
                }
            }
            out.push(residual);
        }
        Some(out)
    }
}
impl FastPredicate for ContributionTable {
    fn residual(&self, mask: &AliveMask) -> Option<i128> {
        if !self.linear {
            return None;
        }
        let sum = self
            .units
            .iter()
            .zip(&self.values)
            .filter(|(unit, _)| mask.contains(**unit as usize))
            .try_fold(0_i128, |sum, (_, value)| sum.checked_add(*value))?;
        self.constant.checked_sub(sum)
    }
    fn prefix_residual(&self, end: usize) -> Option<i128> {
        self.linear.then_some(())?;
        self.constant.checked_sub(*self.prefix.get(end)?)
    }
}
