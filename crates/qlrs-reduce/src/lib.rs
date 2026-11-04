//! Lifecycle-unit reduction with CSR closures, exact predicates and certificates.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
use qlrs_check::{Checker, Finding, InstanceKey, Invariant, Report};
use qlrs_ledger::{ExpectedState, Ledger};
use qlrs_num::Fixed;
use qlrs_schema::{
    AliveMask, CheckpointPosition, Kind, Side, Status, SyntheticCheckpoint, TraceStore,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, VecDeque};

/// Dependency-to-dependent CSR graph, plus reverse CSR for pin closure.
#[derive(Debug, Clone)]
pub struct UnitGraph {
    #[doc = "offsets."]
    pub offsets: Vec<u32>,
    #[doc = "targets."]
    pub targets: Vec<u32>,
    #[doc = "parent offsets."]
    pub parent_offsets: Vec<u32>,
    #[doc = "parents."]
    pub parents: Vec<u32>,
}
impl UnitGraph {
    /// Conservatively connect sell units to enough preceding buy units.
    pub fn build(store: &TraceStore) -> Result<Self, String> {
        let n = store.units.len();
        let mut edges = vec![BTreeSet::new(); n];
        let mut buys: Vec<VecDeque<(u32, i128)>> =
            vec![VecDeque::new(); store.header.instruments.len()];
        for event in 0..store.len() {
            if store.spine.kind.get(event) != Some(&Kind::Filled) {
                continue;
            }
            let Some(fill) = store.spine.payload_idx.get(event).map(|v| *v as usize) else {
                continue;
            };
            let Some(order) = store.fills.order.get(fill).map(|v| *v as usize) else {
                continue;
            };
            let Some(instrument) = store.orders.instrument.get(order).map(|v| *v as usize) else {
                continue;
            };
            let Some(unit) = store.orders.unit.get(order).copied() else {
                continue;
            };
            let quantity = store.fill_qty.get(fill).copied().unwrap_or(0);
            let Some(history) = buys.get_mut(instrument) else {
                continue;
            };
            match store.orders.side.get(order) {
                Some(Side::Buy) => history.push_back((unit, quantity)),
                Some(Side::Sell) => {
                    let mut needed = quantity;
                    while needed > 0 {
                        let Some((buy, qty)) = history.pop_front() else {
                            break;
                        };
                        let consumed = needed.min(qty);
                        if buy != unit {
                            edges
                                .get_mut(buy as usize)
                                .ok_or("invalid buy unit")?
                                .insert(unit);
                        }
                        needed = needed
                            .checked_sub(consumed)
                            .ok_or("dependency quantity overflow")?;
                        let remaining = qty
                            .checked_sub(consumed)
                            .ok_or("dependency quantity overflow")?;
                        if remaining > 0 {
                            history.push_front((buy, remaining));
                        }
                    }
                }
                None => {}
            }
        }
        let mut reverse = vec![BTreeSet::new(); n];
        for (from, targets) in edges.iter().enumerate() {
            for &to in targets {
                reverse
                    .get_mut(to as usize)
                    .ok_or("invalid dependency")?
                    .insert(u32::try_from(from).map_err(|_| "too many units")?);
            }
        }
        let (offsets, targets) = csr(edges)?;
        let (parent_offsets, parents) = csr(reverse)?;
        Ok(Self {
            offsets,
            targets,
            parent_offsets,
            parents,
        })
    }
    /// A deterministic transitive removal closure including all dependents.
    #[must_use]
    pub fn closure(&self, roots: &[usize]) -> Vec<usize> {
        walk(&self.offsets, &self.targets, roots)
    }
    /// Transitive prerequisites that must be kept with pinned units.
    #[must_use]
    pub fn prerequisites(&self, roots: &[usize]) -> Vec<usize> {
        walk(&self.parent_offsets, &self.parents, roots)
    }
}
fn csr(rows: Vec<BTreeSet<u32>>) -> Result<(Vec<u32>, Vec<u32>), String> {
    let mut offsets = Vec::with_capacity(rows.len() + 1);
    let mut targets = Vec::new();
    offsets.push(0);
    for row in rows {
        targets.extend(row);
        offsets.push(u32::try_from(targets.len()).map_err(|_| "CSR exceeds u32")?);
    }
    Ok((offsets, targets))
}
fn walk(offsets: &[u32], targets: &[u32], roots: &[usize]) -> Vec<usize> {
    let mut seen = AliveMask::empty(offsets.len().saturating_sub(1));
    let mut stack = roots.to_vec();
    while let Some(unit) = stack.pop() {
        if seen.contains(unit) {
            continue;
        }
        seen.set(unit, true);
        let start = offsets.get(unit).copied().unwrap_or(0) as usize;
        let end = offsets.get(unit.saturating_add(1)).copied().unwrap_or(0) as usize;
        for &to in targets.get(start..end).unwrap_or_default() {
            stack.push(to as usize);
        }
    }
    seen.iter().collect()
}
/// Every predicate invocation is logged, including cached and algebraic rejects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    #[doc = "candidate digest."]
    pub candidate_digest: String,
    #[doc = "units removed."]
    pub units_removed: Vec<u32>,
    #[doc = "result."]
    pub result: bool,
    #[doc = "cache hit."]
    pub cache_hit: bool,
    #[doc = "full replay."]
    pub full_replay: bool,
}
/// A deterministic predicate result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    #[doc = "accepted."]
    pub accepted: bool,
    #[doc = "digest."]
    pub digest: String,
    #[doc = "cache hit."]
    pub cache_hit: bool,
}
/// Stable predicate interface for Part 2's search algorithms.
pub trait Predicate {
    /// Test one complete lifecycle-unit mask.
    fn test(&mut self, alive: &AliveMask) -> Result<Outcome, String>;
}

struct UnitEvents {
    offsets: Vec<u32>,
    events: Vec<u32>,
}
impl UnitEvents {
    fn new(store: &TraceStore) -> Result<Self, String> {
        let mut offsets = vec![0_u32; store.units.len().saturating_add(1)];
        for &unit in &store.spine.unit {
            let count = offsets
                .get_mut((unit as usize).saturating_add(1))
                .ok_or("invalid event unit")?;
            *count = count.checked_add(1).ok_or("too many unit events")?;
        }
        for unit in 1..offsets.len() {
            let previous = offsets
                .get(unit.saturating_sub(1))
                .copied()
                .ok_or("invalid offsets")?;
            let value = offsets.get_mut(unit).ok_or("invalid offsets")?;
            *value = value
                .checked_add(previous)
                .ok_or("unit index exceeds u32")?;
        }
        let mut next = offsets.clone();
        let mut events = vec![0_u32; store.len()];
        for (event, &unit) in store.spine.unit.iter().enumerate() {
            let slot = next.get_mut(unit as usize).ok_or("invalid event unit")?;
            *events.get_mut(*slot as usize).ok_or("invalid unit index")? =
                u32::try_from(event).map_err(|_| "event index exceeds u32")?;
            *slot = slot.checked_add(1).ok_or("unit offset exceeds u32")?;
        }
        Ok(Self { offsets, events })
    }
    fn count(&self, alive: &AliveMask) -> usize {
        alive.iter().fold(0_usize, |total, unit| {
            let start = self.offsets.get(unit).copied().unwrap_or(0);
            let end = self
                .offsets
                .get(unit.saturating_add(1))
                .copied()
                .unwrap_or(0);
            total.saturating_add(end.saturating_sub(start) as usize)
        })
    }
    fn kept(&self, alive: &AliveMask) -> Vec<usize> {
        let mut events = Vec::with_capacity(self.count(alive));
        for unit in alive.iter() {
            let start = self.offsets.get(unit).copied().unwrap_or(0) as usize;
            let end = self
                .offsets
                .get(unit.saturating_add(1))
                .copied()
                .unwrap_or(0) as usize;
            events.extend(
                self.events
                    .get(start..end)
                    .unwrap_or_default()
                    .iter()
                    .map(|event| *event as usize),
            );
        }
        events.sort_unstable();
        events
    }
}

struct PrefixIndex {
    snapshots: Vec<(usize, ExpectedState)>,
    initial: Option<SyntheticCheckpoint>,
    digest: String,
}
impl PrefixIndex {
    fn new(
        store: &TraceStore,
        checkpoint: Option<&SyntheticCheckpoint>,
        digest: &str,
    ) -> Result<Self, String> {
        let mut state = Ledger::initial(store, checkpoint)?;
        let mut snapshots = vec![(0, state.clone())];
        for event in 0..store.len() {
            state
                .apply(store, event)
                .map_err(|e| format!("prefix {} at {}", e.reason, e.seq))?;
            if event.saturating_add(1) % 4096 == 0 {
                snapshots.push((event.saturating_add(1), state.clone()));
            }
        }
        Ok(Self {
            snapshots,
            initial: checkpoint.cloned(),
            digest: digest.into(),
        })
    }
    fn checkpoint(
        &self,
        store: &TraceStore,
        alive: &AliveMask,
    ) -> Result<Option<SyntheticCheckpoint>, String> {
        let Some(first) = alive
            .iter()
            .filter_map(|unit| store.units.get(unit).map(|u| u.first_event))
            .min()
        else {
            return Err("empty witness".into());
        };
        if first == 0 {
            return Ok(self.initial.clone());
        }
        let (start, snapshot) = self
            .snapshots
            .get(first / 4096)
            .ok_or("missing prefix snapshot")?;
        let mut state = snapshot.clone();
        for event in *start..first {
            state
                .apply(store, event)
                .map_err(|e| format!("prefix {}", e.reason))?;
        }
        let positions = store
            .header
            .instruments
            .iter()
            .zip(&state.positions)
            .map(|(i, raw)| {
                Ok(CheckpointPosition {
                    instrument_id: i.id.clone(),
                    net_qty: Fixed::new(*raw, i.qty_precision)
                        .map_err(|e| e.to_string())?
                        .to_string(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Some(SyntheticCheckpoint {
            provenance: "synthetic".into(),
            account_id: store.account().into(),
            currency: store.header.model.base_currency.clone(),
            cash: Fixed::new(state.cash, store.cash_scale)
                .map_err(|e| e.to_string())?
                .to_string(),
            positions,
            cut_seq: *store.spine.seq.get(first).ok_or("missing first seq")?,
            source_digest: self.digest.clone(),
            description: format!(
                "computed by qlrs ledger from original trace {}",
                self.digest
            ),
        }))
    }
}
/// Full-replay predicate with exact target identity and anti-slippage checks.
pub struct ReductionPredicate<'a> {
    store: &'a TraceStore,
    events: UnitEvents,
    target: InstanceKey,
    original: BTreeSet<InstanceKey>,
    prefix: PrefixIndex,
    cache: HashMap<[u8; 32], bool, foldhash::fast::RandomState>,
    log: Vec<LogEntry>,
    all: AliveMask,
}
impl<'a> ReductionPredicate<'a> {
    /// Build from independently checked original evidence.
    pub fn new(
        store: &'a TraceStore,
        target: InstanceKey,
        original: &Report,
        checkpoint: Option<&SyntheticCheckpoint>,
        source_digest: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            store,
            events: UnitEvents::new(store)?,
            target,
            original: original.keys().cloned().collect(),
            prefix: PrefixIndex::new(store, checkpoint, source_digest)?,
            cache: HashMap::default(),
            log: Vec::new(),
            all: AliveMask::all(store.units.len()),
        })
    }
    fn full(&self, mask: &AliveMask) -> Result<bool, String> {
        if !self
            .store
            .spine
            .seq
            .binary_search(&self.target.anchor_claim_original_seq)
            .ok()
            .and_then(|e| self.store.spine.unit.get(e))
            .is_some_and(|u| mask.contains(*u as usize))
        {
            return Ok(false);
        }
        let checkpoint = self.prefix.checkpoint(self.store, mask)?;
        let result = Checker::check(self.store, mask, checkpoint.as_ref(), true);
        Ok(result.keys().any(|k| *k == self.target)
            && result
                .findings
                .iter()
                .all(|f| f.status != Status::Unsupported && f.status != Status::Inconclusive)
            && result.keys().all(|key| self.original.contains(key))
            && result
                .keys()
                .filter(|key| key.invariant_id == Invariant::I3)
                .all(|key| *key == self.target))
    }
    // Complete sparse I1 admission from precomputed fill deltas and claim columns.
    // Deleting whole originally legal lifecycles cannot create an I3 breach;
    // sources with any I3 finding use the full checker instead.
    fn fast_candidate(&self, mask: &AliveMask) -> Result<Option<bool>, String> {
        if self.target.invariant_id != Invariant::I1
            || self
                .original
                .iter()
                .any(|key| key.invariant_id == Invariant::I3)
        {
            return Ok(None);
        }
        if !self
            .store
            .spine
            .seq
            .binary_search(&self.target.anchor_claim_original_seq)
            .ok()
            .and_then(|e| self.store.spine.unit.get(e))
            .is_some_and(|u| mask.contains(*u as usize))
        {
            return Ok(Some(false));
        }
        if self.events.count(mask) > self.store.len() / 4 {
            return Ok(None);
        }
        let events = self.events.kept(mask);
        let checkpoint = self.prefix.checkpoint(self.store, mask)?;
        let mut state = Ledger::initial(self.store, checkpoint.as_ref())?;
        let mut base = state.cash;
        let mut delta = 0_i128;
        let mut cash_claims = 0_usize;
        let mut filled = vec![false; state.positions.len()];
        let mut claimed = vec![false; state.positions.len()];
        let mut target = false;
        for event in events {
            let Some(kind) = self.store.spine.kind.get(event).copied() else {
                return Ok(None);
            };
            let Some(payload) = self.store.spine.payload_idx.get(event).map(|p| *p as usize) else {
                return Ok(None);
            };
            let Some(seq) = self.store.spine.seq.get(event).copied() else {
                return Ok(None);
            };
            match kind {
                Kind::Filled => {
                    if state.apply(self.store, event).is_err() {
                        return Ok(Some(false));
                    }
                    let Some(sum) = self
                        .store
                        .delta_cash
                        .get(event)
                        .and_then(|raw| delta.checked_add(*raw))
                    else {
                        return Ok(Some(false));
                    };
                    delta = sum;
                    let instrument =
                        self.store
                            .fills
                            .order
                            .get(payload)
                            .and_then(|order| self.store.orders.instrument.get(*order as usize))
                            .copied()
                            .ok_or("invalid fill reference")? as usize;
                    *filled
                        .get_mut(instrument)
                        .ok_or("invalid instrument reference")? = true;
                }
                Kind::CashClaim => {
                    cash_claims = cash_claims.saturating_add(1);
                    let raw = *self
                        .store
                        .cash_claims
                        .raw
                        .get(payload)
                        .ok_or("invalid cash claim")?;
                    let scale = *self
                        .store
                        .cash_claims
                        .scale
                        .get(payload)
                        .ok_or("invalid cash scale")?;
                    let observed = Fixed::new(raw, scale)
                        .and_then(|v| v.rescale(self.store.cash_scale, qlrs_num::Rounding::Exact));
                    let expected = base
                        .checked_add(delta)
                        .ok_or(qlrs_num::NumericError::Overflow)
                        .and_then(|raw| Fixed::new(raw, self.store.cash_scale))
                        .and_then(|v| {
                            v.rescale(
                                self.store
                                    .header
                                    .currencies
                                    .first()
                                    .map_or(0, |c| c.precision),
                                self.store.header.model.engine_cash_rounding.numeric(),
                            )
                        })
                        .and_then(|v| v.rescale(self.store.cash_scale, qlrs_num::Rounding::Exact));
                    let (Ok(observed), Ok(expected)) = (observed, expected) else {
                        return Ok(Some(false));
                    };
                    let Some(residual) = observed.raw().checked_sub(expected.raw()) else {
                        return Ok(Some(false));
                    };
                    if residual != 0 {
                        if !self.known_residual(
                            Invariant::I1,
                            "cash-conservation",
                            &self.store.header.model.base_currency,
                            seq,
                            residual,
                            self.store.cash_scale,
                        ) {
                            return Ok(Some(false));
                        }
                        if seq == self.target.anchor_claim_original_seq
                            && self.target.residual_raw == residual.to_string()
                            && self.target.residual_scale == self.store.cash_scale
                        {
                            target = true;
                        }
                    }
                    base = observed.raw();
                    delta = 0;
                }
                Kind::PositionClaim => {
                    let instrument = *self
                        .store
                        .position_claims
                        .instrument
                        .get(payload)
                        .ok_or("invalid position claim")?
                        as usize;
                    *claimed
                        .get_mut(instrument)
                        .ok_or("invalid position instrument")? = true;
                    let metadata = self
                        .store
                        .header
                        .instruments
                        .get(instrument)
                        .ok_or("invalid position metadata")?;
                    let observed = Fixed::new(
                        *self
                            .store
                            .position_claims
                            .raw
                            .get(payload)
                            .ok_or("invalid position raw")?,
                        *self
                            .store
                            .position_claims
                            .scale
                            .get(payload)
                            .ok_or("invalid position scale")?,
                    )
                    .and_then(|v| v.rescale(metadata.qty_precision, qlrs_num::Rounding::Exact));
                    let Ok(observed) = observed else {
                        return Ok(Some(false));
                    };
                    let Some(residual) = state
                        .positions
                        .get(instrument)
                        .and_then(|expected| observed.raw().checked_sub(*expected))
                    else {
                        return Ok(Some(false));
                    };
                    if residual != 0
                        && !self.known_residual(
                            Invariant::I2,
                            "position-sum",
                            &metadata.id,
                            seq,
                            residual,
                            metadata.qty_precision,
                        )
                    {
                        return Ok(Some(false));
                    }
                }
                _ => {}
            }
        }
        if filled.iter().any(|v| *v) && cash_claims == 0
            || filled.iter().zip(&claimed).any(|(f, c)| *f && !*c)
        {
            return Ok(Some(false));
        }
        Ok(Some(target))
    }
    fn known_residual(
        &self,
        invariant: Invariant,
        code: &str,
        scope: &str,
        seq: u64,
        raw: i128,
        scale: u8,
    ) -> bool {
        let raw = raw.to_string();
        self.original.iter().any(|key| {
            key.invariant_id == invariant
                && key.sub_code == code
                && key.scope == scope
                && key.anchor_claim_original_seq == seq
                && key.residual_raw == raw
                && key.residual_scale == scale
        })
    }

    fn batch(&mut self, masks: &[AliveMask]) -> Result<Vec<bool>, String> {
        let hashes: Vec<_> = masks.iter().map(mask_hash).collect();
        let known: Vec<_> = hashes.iter().map(|h| self.cache.get(h).copied()).collect();
        let results: Vec<Result<(bool, bool), String>> = masks
            .par_iter()
            .zip(&known)
            .map(|(mask, cached)| {
                if let Some(value) = cached {
                    Ok((*value, false))
                } else if let Some(outcome) = self.fast_candidate(mask)? {
                    Ok((outcome, false))
                } else {
                    self.full(mask).map(|outcome| (outcome, true))
                }
            })
            .collect();
        let mut out = Vec::with_capacity(masks.len());
        for (((mask, hash), cached), result) in masks.iter().zip(hashes).zip(known).zip(results) {
            let (result, full_replay) = result?;
            self.cache.insert(hash, result);
            self.log.push(LogEntry {
                candidate_digest: hex(&hash),
                units_removed: self
                    .all
                    .iter()
                    .filter(|id| !mask.contains(*id))
                    .filter_map(|id| u32::try_from(id).ok())
                    .collect(),
                result,
                cache_hit: cached.is_some(),
                full_replay,
            });
            out.push(result);
        }
        Ok(out)
    }
}
impl Predicate for ReductionPredicate<'_> {
    fn test(&mut self, alive: &AliveMask) -> Result<Outcome, String> {
        let digest = mask_hash(alive);
        let cached = self.cache.contains_key(&digest);
        let result = self
            .batch(std::slice::from_ref(alive))?
            .first()
            .copied()
            .unwrap_or(false);
        Ok(Outcome {
            accepted: result,
            digest: hex(&digest),
            cache_hit: cached,
        })
    }
}
fn mask_hash(mask: &AliveMask) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(&u64::try_from(mask.len()).unwrap_or(u64::MAX).to_le_bytes());
    for word in mask.words() {
        hash.update(&word.to_le_bytes());
    }
    *hash.finalize().as_bytes()
}
fn hex(hash: &[u8; 32]) -> String {
    blake3::Hash::from(*hash).to_hex().to_string()
}

/// A replayable closure-1-minimal witness, never a global minimum claim.
#[derive(Debug)]
pub struct Reduction {
    #[doc = "alive."]
    pub alive: AliveMask,
    #[doc = "checkpoint."]
    pub checkpoint: Option<SyntheticCheckpoint>,
    #[doc = "target."]
    pub target: Finding,
    #[doc = "log."]
    pub log: Vec<LogEntry>,
    #[doc = "closure 1 minimal."]
    pub closure_1_minimal: bool,
    #[doc = "retained events."]
    pub retained_events: usize,
}
/// Reduce a violating trace using pinned slicing, deterministic parallel ddmin,
/// then an actual single-closure minimality pass and a final full replay.
pub fn reduce(
    store: &TraceStore,
    source_digest: &str,
    checkpoint: Option<&SyntheticCheckpoint>,
    target_seq: Option<u64>,
) -> Result<Reduction, String> {
    let original = Checker::check(store, &AliveMask::all(store.units.len()), checkpoint, true);
    if original.status != Status::Violation
        || original
            .findings
            .iter()
            .any(|f| matches!(f.status, Status::Unsupported | Status::Inconclusive))
    {
        return Err("reduction requires complete supported violation evidence".into());
    }
    let target = if let Some(seq) = target_seq {
        original.at_seq(seq)
    } else {
        original
            .findings
            .iter()
            .find(|f| f.status == Status::Violation)
    }
    .cloned()
    .ok_or("no target violation")?;
    let key = target
        .instance_key
        .clone()
        .ok_or("target lacks instance key")?;
    let graph = UnitGraph::build(store)?;
    let pinned = pinned_units(store, &target)?;
    let roots: Vec<_> = pinned.iter().collect();
    let required = graph.prerequisites(&roots);
    let mut sliced = AliveMask::empty(store.units.len());
    for id in required {
        sliced.set(id, true);
    }
    let mut predicate =
        ReductionPredicate::new(store, key.clone(), &original, checkpoint, source_digest)?;
    let mut current = AliveMask::all(store.units.len());
    if predicate.test(&sliced)?.accepted {
        current = sliced;
    }
    let mut granularity = 2_usize;
    loop {
        let removable: Vec<_> = current.iter().filter(|id| !pinned.contains(*id)).collect();
        if removable.len() < 2 {
            break;
        }
        let size = removable.len().div_ceil(granularity);
        let chunks: Vec<_> = removable.chunks(size).collect();
        let mut candidates = Vec::new();
        for chunk in &chunks {
            if let Some(mask) = remove_closure(&current, &graph, chunk, &pinned) {
                candidates.push(mask);
            }
        }
        for chunk in &chunks {
            let keep: BTreeSet<_> = chunk.iter().copied().collect();
            let remove: Vec<_> = removable
                .iter()
                .copied()
                .filter(|u| !keep.contains(u))
                .collect();
            if let Some(mask) = remove_closure(&current, &graph, &remove, &pinned) {
                candidates.push(mask);
            }
        }
        let results = predicate.batch(&candidates)?;
        if let Some(index) = results.iter().position(|pass| *pass) {
            current = candidates
                .get(index)
                .ok_or("missing deterministic candidate")?
                .clone();
            granularity = granularity.saturating_sub(1).max(2);
        } else if granularity >= removable.len() {
            break;
        } else {
            granularity = granularity.saturating_mul(2).min(removable.len());
        }
    }
    loop {
        let mut changed = false;
        let remaining: Vec<_> = current.iter().filter(|id| !pinned.contains(*id)).collect();
        for unit in remaining {
            if let Some(candidate) = remove_closure(&current, &graph, &[unit], &pinned)
                && predicate.test(&candidate)?.accepted
            {
                current = candidate;
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    let final_full = predicate.full(&current)?;
    let fast = predicate.fast_candidate(&current)?;
    if fast.is_some_and(|value| value != final_full) {
        return Err("BUG: fast and full predicate disagree".into());
    }
    if !final_full {
        return Err("no witness satisfies lifecycle legality and anti-slippage".into());
    }
    predicate.log.push(LogEntry {
        candidate_digest: hex(&mask_hash(&current)),
        units_removed: predicate
            .all
            .iter()
            .filter(|u| !current.contains(*u))
            .filter_map(|u| u32::try_from(u).ok())
            .collect(),
        result: true,
        cache_hit: false,
        full_replay: true,
    });
    let final_checkpoint = predicate.prefix.checkpoint(store, &current)?;
    let final_report = Checker::check(store, &current, final_checkpoint.as_ref(), true);
    let final_target = final_report
        .findings
        .into_iter()
        .find(|f| f.instance_key.as_ref() == Some(&key))
        .ok_or("final target missing")?;
    let retained_events = store
        .spine
        .unit
        .iter()
        .filter(|u| current.contains(**u as usize))
        .count();
    Ok(Reduction {
        alive: current,
        checkpoint: final_checkpoint,
        target: final_target,
        log: predicate.log,
        closure_1_minimal: true,
        retained_events,
    })
}
fn remove_closure(
    current: &AliveMask,
    graph: &UnitGraph,
    roots: &[usize],
    pinned: &AliveMask,
) -> Option<AliveMask> {
    let closure = graph.closure(roots);
    if closure.iter().any(|id| pinned.contains(*id)) {
        return None;
    }
    let mut result = current.clone();
    for id in closure {
        result.set(id, false);
    }
    if result == *current {
        None
    } else {
        Some(result)
    }
}
/// Pinned evidence units under the v0 lifecycle and completeness rules.
pub fn pinned_units(store: &TraceStore, target: &Finding) -> Result<AliveMask, String> {
    let key = target.instance_key.as_ref().ok_or("missing instance key")?;
    let mut pinned = AliveMask::empty(store.units.len());
    let anchor = store
        .spine
        .seq
        .iter()
        .position(|seq| *seq == key.anchor_claim_original_seq)
        .ok_or("missing anchor")?;
    let unit = *store.spine.unit.get(anchor).ok_or("missing anchor unit")?;
    pinned.set(unit as usize, true);
    if key.invariant_id == Invariant::I1 {
        for event in 0..=anchor {
            let seq = *store.spine.seq.get(event).ok_or("invalid spine")?;
            let unit = *store.spine.unit.get(event).ok_or("invalid spine")? as usize;
            if target.window.start_seq == Some(seq)
                || (target.window.start_seq.is_none_or(|s| seq > s)
                    && store.units.get(unit).is_some_and(|u| u.order.is_some()))
            {
                pinned.set(unit, true);
            }
        }
    }
    // Pin evidence required to avoid turning a small complete trace into an
    // inconclusive one: one account and one post-fill position claim per asset.
    let last_fill = (0..store.len())
        .filter(|e| {
            store.spine.kind.get(*e) == Some(&Kind::Filled)
                && store
                    .spine
                    .unit
                    .get(*e)
                    .is_some_and(|u| pinned.contains(*u as usize))
        })
        .max();
    if let Some(last) = last_fill {
        if let Some(event) =
            (last..store.len()).find(|e| store.spine.kind.get(*e) == Some(&Kind::CashClaim))
            && let Some(unit) = store.spine.unit.get(event)
        {
            pinned.set(*unit as usize, true);
        }
        let mut instruments = BTreeSet::new();
        for event in 0..=last {
            if store.spine.kind.get(event) == Some(&Kind::Filled)
                && store
                    .spine
                    .unit
                    .get(event)
                    .is_some_and(|u| pinned.contains(*u as usize))
                && let Some(instrument) = store
                    .spine
                    .payload_idx
                    .get(event)
                    .and_then(|f| store.fills.order.get(*f as usize))
                    .and_then(|o| store.orders.instrument.get(*o as usize))
            {
                instruments.insert(*instrument);
            }
        }
        for instrument in instruments {
            if let Some(event) = (last..store.len()).find(|e| {
                store.spine.kind.get(*e) == Some(&Kind::PositionClaim)
                    && store
                        .spine
                        .payload_idx
                        .get(*e)
                        .and_then(|p| store.position_claims.instrument.get(*p as usize))
                        == Some(&instrument)
            }) && let Some(unit) = store.spine.unit.get(event)
            {
                pinned.set(*unit as usize, true);
            }
        }
    }
    Ok(pinned)
}
