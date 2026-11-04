//! Claim-boundary lifecycle reduction with bounded deterministic search.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
use qlrs_check::{Checker, Finding, InstanceKey, Invariant, Report};
use qlrs_ledger::Ledger;
use qlrs_schema::{AliveMask, Kind, Side, Status, SyntheticCheckpoint, TraceStore};
use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    time::{Duration, Instant},
};

/// Dependency-to-dependent CSR graph, with reverse prerequisite edges.
#[derive(Debug, Clone)]
pub struct UnitGraph {
    /// Forward row offsets.
    pub offsets: Vec<u32>,
    /// Forward targets.
    pub targets: Vec<u32>,
    /// Reverse row offsets.
    pub parent_offsets: Vec<u32>,
    /// Reverse targets.
    pub parents: Vec<u32>,
}
impl UnitGraph {
    /// Build full-trace FIFO inventory dependencies.
    pub fn build(store: &TraceStore) -> Result<Self, String> {
        Self::build_with_checkpoint(store, None, &AliveMask::all(store.units.len()))
    }
    /// Consume checkpoint inventory before post-cut buys. Prefix buys cannot
    /// become prerequisites of retained sells once their inventory is in state.
    pub fn build_with_checkpoint(
        store: &TraceStore,
        checkpoint: Option<&SyntheticCheckpoint>,
        window: &AliveMask,
    ) -> Result<Self, String> {
        let mut inventory = Ledger::initial(store, checkpoint)?.positions;
        let mut buys = vec![VecDeque::<(u32, i128)>::new(); inventory.len()];
        let mut edges = Vec::new();
        for event in 0..store.len() {
            if !window.contains_event(store, event)
                || store.spine.kind.get(event) != Some(&Kind::Filled)
            {
                continue;
            }
            let fill = *store.spine.payload_idx.get(event).ok_or("invalid fill")? as usize;
            let order = *store.fills.order.get(fill).ok_or("invalid order")? as usize;
            let instrument = *store
                .orders
                .instrument
                .get(order)
                .ok_or("invalid instrument")? as usize;
            let unit = *store.orders.unit.get(order).ok_or("invalid unit")?;
            let quantity = *store.fill_qty.get(fill).ok_or("unprepared fill")?;
            let history = buys.get_mut(instrument).ok_or("invalid instrument")?;
            match store.orders.side.get(order) {
                Some(Side::Buy) => history.push_back((unit, quantity)),
                Some(Side::Sell) => {
                    let opening = inventory.get_mut(instrument).ok_or("invalid inventory")?;
                    let covered = quantity.min(*opening);
                    *opening = opening.checked_sub(covered).ok_or("inventory overflow")?;
                    let mut needed = quantity.checked_sub(covered).ok_or("inventory overflow")?;
                    let mut remainder = None;
                    while needed > 0 {
                        let Some((buy, qty)) = history.pop_front() else {
                            return Err("sell inventory not covered by checkpoint or buys".into());
                        };
                        let consumed = needed.min(qty);
                        if buy != unit {
                            edges.push((buy, unit));
                        }
                        needed = needed
                            .checked_sub(consumed)
                            .ok_or("dependency quantity overflow")?;
                        let remaining = qty
                            .checked_sub(consumed)
                            .ok_or("dependency quantity overflow")?;
                        if remaining > 0 {
                            remainder = Some((buy, remaining));
                        }
                    }
                    // Each iteration permanently consumes one queue entry.
                    // Return a partially consumed lot only after the loop, so
                    // even a zero lot can never be repeatedly re-enqueued.
                    if let Some(lot) = remainder {
                        history.push_front(lot);
                    }
                }
                None => return Err("missing side".into()),
            }
        }
        edges.sort_unstable();
        edges.dedup();
        let (offsets, targets) = csr(store.units.len(), &edges)?;
        let mut reverse: Vec<_> = edges.into_iter().map(|(a, b)| (b, a)).collect();
        reverse.sort_unstable();
        let (parent_offsets, parents) = csr(store.units.len(), &reverse)?;
        Ok(Self {
            offsets,
            targets,
            parent_offsets,
            parents,
        })
    }
    /// Sorted transitive removal closure, including the roots.
    #[must_use]
    pub fn closure(&self, roots: &[usize]) -> Vec<usize> {
        walk(&self.offsets, &self.targets, roots)
    }
    /// Sorted transitive prerequisites, including the roots.
    #[must_use]
    pub fn prerequisites(&self, roots: &[usize]) -> Vec<usize> {
        walk(&self.parent_offsets, &self.parents, roots)
    }
}
fn csr(units: usize, edges: &[(u32, u32)]) -> Result<(Vec<u32>, Vec<u32>), String> {
    let mut offsets = vec![0_u32; units.checked_add(1).ok_or("too many units")?];
    for &(from, _) in edges {
        let count = offsets
            .get_mut((from as usize).saturating_add(1))
            .ok_or("invalid edge")?;
        *count = count.checked_add(1).ok_or("too many edges")?;
    }
    for i in 1..offsets.len() {
        let previous = *offsets.get(i.saturating_sub(1)).ok_or("invalid CSR")?;
        let value = offsets.get_mut(i).ok_or("invalid CSR")?;
        *value = value.checked_add(previous).ok_or("too many edges")?;
    }
    Ok((offsets, edges.iter().map(|(_, to)| *to).collect()))
}
fn walk(offsets: &[u32], targets: &[u32], roots: &[usize]) -> Vec<usize> {
    let mut seen = AliveMask::empty(offsets.len().saturating_sub(1));
    let mut stack = roots.to_vec();
    while let Some(unit) = stack.pop() {
        if unit >= seen.len() || seen.contains(unit) {
            continue;
        }
        seen.set(unit, true);
        let start = offsets.get(unit).copied().unwrap_or(0) as usize;
        let end = offsets.get(unit.saturating_add(1)).copied().unwrap_or(0) as usize;
        stack.extend(
            targets
                .get(start..end)
                .unwrap_or_default()
                .iter()
                .map(|id| *id as usize),
        );
    }
    seen.iter().collect()
}

/// Constant-size predicate-call summary. Legacy removed-ID lists remain readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    /// Candidate mask commitment (includes its observation bounds).
    pub candidate_digest: String,
    /// Removed-unit count; no list proportional to original history.
    #[serde(deserialize_with = "removed_count")]
    pub units_removed: u64,
    /// Predicate admission.
    pub result: bool,
    /// Reused admission result.
    pub cache_hit: bool,
    /// Complete invariant/precondition replay, including sparse selected replay.
    pub full_replay: bool,
}
fn removed_count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Count {
        Number(u64),
        Legacy(Vec<u32>),
    }
    Ok(match Count::deserialize(deserializer)? {
        Count::Number(n) => n,
        Count::Legacy(ids) => u64::try_from(ids.len()).unwrap_or(u64::MAX),
    })
}
/// Exact admission result for one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Whether the complete predicate admits the candidate.
    pub accepted: bool,
    /// Candidate commitment.
    pub digest: String,
    /// Whether admission came from cache.
    pub cache_hit: bool,
}
/// Fallible predicate API for further search strategies.
pub trait Predicate {
    /// Evaluate one unit mask under this predicate's fixed claim boundary.
    fn test(&mut self, alive: &AliveMask) -> Result<Outcome, String>;
}
/// Cooperative reduction budgets, including cache-hit candidate calls.
#[derive(Debug, Clone, Copy)]
pub struct ReductionLimits {
    /// Maximum candidate evaluations; input validation is mandatory and separate.
    pub max_calls: u64,
    /// Elapsed search/setup budget; an already-started complete replay finishes.
    pub time_limit: Duration,
}
impl Default for ReductionLimits {
    fn default() -> Self {
        Self {
            max_calls: 10_000,
            time_limit: Duration::from_secs(30),
        }
    }
}
/// Why search returned a verified but uncertified witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LimitReason {
    /// Candidate-call budget exhausted.
    CallLimit,
    /// Cooperative elapsed budget exhausted.
    TimeLimit,
}
struct Budget {
    start: Instant,
    limits: ReductionLimits,
    calls: u64,
}
impl Budget {
    fn reason(&self) -> Option<LimitReason> {
        if self.calls >= self.limits.max_calls {
            Some(LimitReason::CallLimit)
        } else if self.start.elapsed() >= self.limits.time_limit {
            Some(LimitReason::TimeLimit)
        } else {
            None
        }
    }
}
struct UnitEvents {
    offsets: Vec<u32>,
    events: Vec<u32>,
}
impl UnitEvents {
    fn new(store: &TraceStore, window: &AliveMask) -> Result<Self, String> {
        let mut offsets = vec![0_u32; store.units.len().saturating_add(1)];
        for event in 0..store.len() {
            if window.contains_event(store, event) {
                let unit = *store.spine.unit.get(event).ok_or("invalid spine")? as usize;
                let count = offsets
                    .get_mut(unit.saturating_add(1))
                    .ok_or("invalid unit")?;
                *count = count.checked_add(1).ok_or("too many events")?;
            }
        }
        for i in 1..offsets.len() {
            let previous = *offsets.get(i.saturating_sub(1)).ok_or("invalid offsets")?;
            let value = offsets.get_mut(i).ok_or("invalid offsets")?;
            *value = value.checked_add(previous).ok_or("too many events")?;
        }
        let mut next = offsets.clone();
        let mut events = vec![0; offsets.last().copied().unwrap_or(0) as usize];
        for event in 0..store.len() {
            if window.contains_event(store, event) {
                let unit = *store.spine.unit.get(event).ok_or("invalid spine")? as usize;
                let slot = next.get_mut(unit).ok_or("invalid unit")?;
                *events.get_mut(*slot as usize).ok_or("invalid index")? =
                    u32::try_from(event).map_err(|_| "too many events")?;
                *slot = slot.checked_add(1).ok_or("too many events")?;
            }
        }
        Ok(Self { offsets, events })
    }
    fn kept(&self, alive: &AliveMask) -> Vec<usize> {
        let mut events = Vec::new();
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
                    .map(|e| *e as usize),
            );
        }
        events.sort_unstable();
        events
    }
}
fn frontier(store: &TraceStore, start: usize, anchor: usize) -> Result<usize, String> {
    let mut cash = false;
    let mut positions = vec![false; store.header.instruments.len()];
    for event in start..store.len() {
        let payload = *store.spine.payload_idx.get(event).ok_or("invalid event")? as usize;
        match store.spine.kind.get(event) {
            Some(Kind::Filled) => {
                cash = true;
                let instrument = store
                    .fills
                    .order
                    .get(payload)
                    .and_then(|order| store.orders.instrument.get(*order as usize))
                    .copied()
                    .ok_or("invalid fill")? as usize;
                *positions.get_mut(instrument).ok_or("invalid instrument")? = true;
            }
            Some(Kind::CashClaim) => cash = false,
            Some(Kind::PositionClaim) => {
                let instrument = *store
                    .position_claims
                    .instrument
                    .get(payload)
                    .ok_or("invalid claim")? as usize;
                *positions.get_mut(instrument).ok_or("invalid instrument")? = false;
            }
            _ => {}
        }
        if event >= anchor && !cash && positions.iter().all(|pending| !pending) {
            return Ok(event);
        }
    }
    Err("missing later claim frontier".into())
}
fn window(
    store: &TraceStore,
    target: &Finding,
    input: Option<&SyntheticCheckpoint>,
) -> Result<(AliveMask, Option<u64>, u64), String> {
    let key = target.instance_key.as_ref().ok_or("missing target key")?;
    let anchor = store
        .spine
        .seq
        .binary_search(&key.anchor_claim_original_seq)
        .map_err(|_| "missing anchor")?;
    let cut = if key.invariant_id == Invariant::I3 {
        (0..anchor)
            .rev()
            .find(|event| {
                matches!(
                    store.spine.kind.get(*event),
                    Some(Kind::CashClaim | Kind::PositionClaim)
                )
            })
            .and_then(|event| store.spine.seq.get(event))
            .copied()
            .or_else(|| input.map(|cp| cp.cut_seq))
    } else {
        target
            .window
            .start_seq
            .or_else(|| input.map(|cp| cp.cut_seq))
    };
    let cut = cut.map(|cut| input.map_or(cut, |cp| cut.max(cp.cut_seq)));
    let first = cut.map_or(0, |seq| {
        store.spine.seq.partition_point(|value| *value < seq)
    });
    let end = frontier(store, first, anchor)?;
    let end_seq = *store.spine.seq.get(end).ok_or("missing end")?;
    let mut mask = AliveMask::empty(store.units.len()).with_range(cut, Some(end_seq));
    for event in first..=end {
        mask.set(
            *store.spine.unit.get(event).ok_or("invalid unit")? as usize,
            true,
        );
    }
    Ok((mask, cut, end_seq))
}
/// Complete sparse replay with exact instance identity and no new contradictions.
pub struct ReductionPredicate<'a> {
    store: &'a TraceStore,
    target: InstanceKey,
    original: BTreeSet<InstanceKey>,
    checkpoint: Option<SyntheticCheckpoint>,
    initial: AliveMask,
    events: UnitEvents,
    cache: HashMap<[u8; 32], (bool, Option<Finding>), foldhash::fast::RandomState>,
    log: Vec<LogEntry>,
}
impl<'a> ReductionPredicate<'a> {
    /// Build fixed claim-boundary context from a checked original trace.
    pub fn new(
        store: &'a TraceStore,
        target: InstanceKey,
        original: &Report,
        input: Option<&SyntheticCheckpoint>,
        source_digest: &str,
    ) -> Result<Self, String> {
        let finding = original
            .findings
            .iter()
            .find(|finding| finding.instance_key.as_ref() == Some(&target))
            .ok_or("missing target finding")?;
        let (initial, cut, end) = window(store, finding, input)?;
        let checkpoint = if let Some(cut) = cut {
            Some(Checker::checkpoint_at_claim(
                store,
                cut,
                end,
                input,
                source_digest,
            )?)
        } else {
            input.cloned()
        };
        let events = UnitEvents::new(store, &initial)?;
        Ok(Self {
            store,
            target,
            original: original.keys().cloned().collect(),
            checkpoint,
            initial,
            events,
            cache: HashMap::default(),
            log: Vec::new(),
        })
    }
    /// Normalize external candidate bits to the fixed observation window.
    #[must_use]
    pub fn candidate_mask(&self, mask: &AliveMask) -> AliveMask {
        self.initial.intersection(mask)
    }
    /// Filter lifecycle/trade context to actual retained suffix references.
    pub fn candidate_checkpoint(
        &self,
        mask: &AliveMask,
    ) -> Result<Option<SyntheticCheckpoint>, String> {
        let Some(mut cp) = self.checkpoint.clone() else {
            return Ok(None);
        };
        cp.orders.retain(|order| {
            self.store
                .order_index(&order.client_order_id)
                .and_then(|index| self.store.orders.unit.get(index))
                .is_some_and(|unit| mask.contains(*unit as usize))
        });
        let mut trades = BTreeSet::new();
        for event in self.events.kept(mask) {
            if self.store.spine.kind.get(event) == Some(&Kind::Filled) {
                let fill = *self
                    .store
                    .spine
                    .payload_idx
                    .get(event)
                    .ok_or("invalid fill")? as usize;
                let instrument = self
                    .store
                    .fills
                    .order
                    .get(fill)
                    .and_then(|order| self.store.orders.instrument.get(*order as usize))
                    .and_then(|instrument| self.store.header.instruments.get(*instrument as usize))
                    .ok_or("invalid instrument")?;
                let trade = self
                    .store
                    .fills
                    .trade
                    .get(fill)
                    .and_then(|symbol| self.store.strings.get(*symbol))
                    .ok_or("invalid trade")?;
                trades.insert((instrument.id.as_str(), trade));
            }
        }
        cp.trades.retain(|trade| {
            trades.contains(&(trade.instrument_id.as_str(), trade.trade_id.as_str()))
        });
        Ok(Some(cp))
    }
    fn report(&self, mask: &AliveMask) -> Result<Report, String> {
        let cp = self.candidate_checkpoint(mask)?;
        let events = self.events.kept(mask);
        Ok(Checker::check_selected(
            self.store,
            mask,
            cp.as_ref(),
            true,
            &events,
        ))
    }
    fn admits(&self, report: &Report) -> bool {
        report.status == Status::Violation
            && report.keys().any(|key| *key == self.target)
            && report.findings.iter().all(|finding| {
                !matches!(finding.status, Status::Unsupported | Status::Inconclusive)
            })
            && report.keys().all(|key| self.original.contains(key))
            && report
                .keys()
                .filter(|key| key.invariant_id == Invariant::I3)
                .all(|key| *key == self.target)
    }
    fn batch(&mut self, masks: &[AliveMask]) -> Result<Vec<bool>, String> {
        let hashes: Vec<_> = masks.iter().map(mask_hash).collect();
        let known: Vec<_> = hashes
            .iter()
            .map(|hash| self.cache.get(hash).cloned())
            .collect();
        let results: Vec<Result<(bool, Option<Finding>), String>> = masks
            .par_iter()
            .zip(&known)
            .map(|(mask, cached)| {
                if let Some(value) = cached {
                    return Ok(value.clone());
                }
                let report = self.report(mask)?;
                let admitted = self.admits(&report);
                let finding = if admitted {
                    report
                        .findings
                        .into_iter()
                        .find(|finding| finding.instance_key.as_ref() == Some(&self.target))
                } else {
                    None
                };
                Ok((admitted, finding))
            })
            .collect();
        let mut out = Vec::with_capacity(masks.len());
        for ((hash, cached), result) in hashes.into_iter().zip(known).zip(results) {
            let value = result?;
            let result = value.0;
            self.cache.insert(hash, value);
            let index = out.len();
            let mask = masks.get(index).ok_or("missing candidate")?;
            self.log.push(log_entry(
                hash,
                self.store.units.len(),
                mask,
                result,
                cached.is_some(),
            ));
            out.push(result);
        }
        Ok(out)
    }
}
impl Predicate for ReductionPredicate<'_> {
    fn test(&mut self, alive: &AliveMask) -> Result<Outcome, String> {
        let mask = self.candidate_mask(alive);
        let hash = mask_hash(&mask);
        let cached = self.cache.contains_key(&hash);
        let accepted = self
            .batch(std::slice::from_ref(&mask))?
            .first()
            .copied()
            .unwrap_or(false);
        Ok(Outcome {
            accepted,
            digest: hex(&hash),
            cache_hit: cached,
        })
    }
}
fn mask_hash(mask: &AliveMask) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(&u64::try_from(mask.len()).unwrap_or(u64::MAX).to_le_bytes());
    for bound in [mask.bounds().0, mask.bounds().1] {
        hash.update(&[u8::from(bound.is_some())]);
        hash.update(&bound.unwrap_or(0).to_le_bytes());
    }
    for word in mask.words() {
        hash.update(&word.to_le_bytes());
    }
    *hash.finalize().as_bytes()
}
fn hex(hash: &[u8; 32]) -> String {
    blake3::Hash::from(*hash).to_hex().to_string()
}
fn log_entry(
    hash: [u8; 32],
    units: usize,
    mask: &AliveMask,
    result: bool,
    cached: bool,
) -> LogEntry {
    LogEntry {
        candidate_digest: hex(&hash),
        units_removed: u64::try_from(units.saturating_sub(mask.count())).unwrap_or(u64::MAX),
        result,
        cache_hit: cached,
        full_replay: !cached,
    }
}
/// Best verified witness and the search's actual certification status.
#[derive(Debug)]
pub struct Reduction {
    /// Unit bits and observation bounds.
    pub alive: AliveMask,
    /// Synthetic state at the preceding claim.
    pub checkpoint: Option<SyntheticCheckpoint>,
    /// Replayed exact target finding.
    pub target: Finding,
    /// Constant-size records for every candidate call.
    pub log: Vec<LogEntry>,
    /// True only after a completed single-closure pass without exhausted limits.
    pub closure_1_minimal: bool,
    /// Observed retained event count.
    pub retained_events: usize,
    /// Actual candidate-call count, including cache hits.
    pub predicate_calls: u64,
    /// Budget exhaustion, if any.
    pub stop_reason: Option<LimitReason>,
}
/// Reduce with the documented default time/call budgets.
pub fn reduce(
    store: &TraceStore,
    digest: &str,
    checkpoint: Option<&SyntheticCheckpoint>,
    target_seq: Option<u64>,
) -> Result<Reduction, String> {
    reduce_with_limits(
        store,
        digest,
        checkpoint,
        target_seq,
        ReductionLimits::default(),
    )
}
/// Return the best verified result even when the cooperative search budget ends.
pub fn reduce_with_limits(
    store: &TraceStore,
    digest: &str,
    checkpoint: Option<&SyntheticCheckpoint>,
    target_seq: Option<u64>,
    limits: ReductionLimits,
) -> Result<Reduction, String> {
    let mut budget = Budget {
        start: Instant::now(),
        limits,
        calls: 0,
    };
    let all = AliveMask::all(store.units.len());
    let original = Checker::check(store, &all, checkpoint, true);
    if original.status != Status::Violation
        || original
            .findings
            .iter()
            .any(|finding| matches!(finding.status, Status::Unsupported | Status::Inconclusive))
    {
        return Err("reduction requires complete supported violation evidence".into());
    }
    let target = if let Some(seq) = target_seq {
        original.at_seq(seq)
    } else {
        original
            .findings
            .iter()
            .find(|finding| finding.status == Status::Violation)
    }
    .cloned()
    .ok_or("no target violation")?;
    let key = target.instance_key.clone().ok_or("missing target key")?;
    if let Some(reason) = budget.reason() {
        if original
            .keys()
            .any(|existing| existing.invariant_id == Invariant::I3 && *existing != key)
        {
            return Err(
                "budget exhausted before any lifecycle-valid target witness was verified".into(),
            );
        }
        return Ok(Reduction {
            alive: all.clone(),
            checkpoint: checkpoint.cloned(),
            target,
            log: vec![log_entry(
                mask_hash(&all),
                store.units.len(),
                &all,
                true,
                false,
            )],
            closure_1_minimal: false,
            retained_events: store.len(),
            predicate_calls: 0,
            stop_reason: Some(reason),
        });
    }
    let mut predicate = ReductionPredicate::new(store, key.clone(), &original, checkpoint, digest)?;
    if let Some(reason) = budget.reason() {
        if original
            .keys()
            .any(|existing| existing.invariant_id == Invariant::I3 && *existing != key)
        {
            return Err(
                "budget exhausted before any lifecycle-valid target witness was verified".into(),
            );
        }
        return Ok(Reduction {
            alive: all.clone(),
            checkpoint: checkpoint.cloned(),
            target,
            log: vec![log_entry(
                mask_hash(&all),
                store.units.len(),
                &all,
                true,
                false,
            )],
            closure_1_minimal: false,
            retained_events: store.len(),
            predicate_calls: 0,
            stop_reason: Some(reason),
        });
    }
    let mut current = predicate.initial.clone();
    // This first candidate preserves the complete original local window; it is
    // verified before becoming the fallback for any later exhausted search.
    let verified = predicate
        .batch(std::slice::from_ref(&current))?
        .first()
        .copied()
        .unwrap_or(false);
    budget.calls = budget.calls.saturating_add(1);
    if !verified {
        return Err("claim-boundary context failed exact original replay".into());
    }
    if budget.reason().is_some() {
        return finish_reduction(predicate, current, &budget, false);
    }
    let pinned = pin_window(store, &target, &current)?;
    let graph = UnitGraph::build_with_checkpoint(store, predicate.checkpoint.as_ref(), &current)?;
    let mut sliced =
        AliveMask::empty(store.units.len()).with_range(current.bounds().0, current.bounds().1);
    for unit in graph.prerequisites(&pinned.iter().collect::<Vec<_>>()) {
        if current.contains(unit) {
            sliced.set(unit, true);
        }
    }
    if budget.reason().is_none() && sliced != current {
        let accepted = predicate
            .batch(std::slice::from_ref(&sliced))?
            .first()
            .copied()
            .unwrap_or(false);
        budget.calls = budget.calls.saturating_add(1);
        if accepted {
            current = sliced;
        }
    }
    let mut granularity = 2_usize;
    'search: while budget.reason().is_none() {
        let removable: Vec<_> = current
            .iter()
            .filter(|unit| !pinned.contains(*unit))
            .collect();
        if removable.len() < 2 {
            break;
        }
        let size = removable.len().div_ceil(granularity);
        let chunks: Vec<_> = removable.chunks(size).collect();
        let mut improved = false;
        for complements in [false, true] {
            let mut batch = Vec::with_capacity(8);
            for (index, chunk) in chunks.iter().enumerate() {
                if budget.reason().is_some() {
                    break 'search;
                }
                let roots = if complements {
                    removable
                        .iter()
                        .copied()
                        .filter(|unit| !chunk.contains(unit))
                        .collect()
                } else {
                    chunk.to_vec()
                };
                if let Some(mask) = remove_closure(&current, &graph, &roots, &pinned) {
                    batch.push(mask);
                }
                if batch.len() == 8 || index.saturating_add(1) == chunks.len() {
                    if let Some(best) = evaluate_batch(&mut predicate, &mut budget, &batch)? {
                        current = best;
                        improved = true;
                        break;
                    }
                    batch.clear();
                }
            }
            if improved {
                break;
            }
        }
        if improved {
            granularity = granularity.saturating_sub(1).max(2);
        } else if granularity >= removable.len() {
            break;
        } else {
            granularity = granularity.saturating_mul(2).min(removable.len());
        }
    }
    let mut certified = false;
    while budget.reason().is_none() {
        let mut improved = false;
        for unit in current
            .iter()
            .filter(|unit| !pinned.contains(*unit))
            .collect::<Vec<_>>()
        {
            if budget.reason().is_some() {
                break;
            }
            if let Some(candidate) = remove_closure(&current, &graph, &[unit], &pinned) {
                let accepted = predicate
                    .batch(std::slice::from_ref(&candidate))?
                    .first()
                    .copied()
                    .unwrap_or(false);
                budget.calls = budget.calls.saturating_add(1);
                if accepted {
                    current = candidate;
                    improved = true;
                    break;
                }
            }
        }
        if budget.reason().is_none() && !improved {
            certified = true;
            break;
        }
    }
    finish_reduction(predicate, current, &budget, certified)
}
fn finish_reduction(
    predicate: ReductionPredicate<'_>,
    current: AliveMask,
    budget: &Budget,
    certified: bool,
) -> Result<Reduction, String> {
    let stop_reason = budget.reason();
    let checkpoint = predicate.candidate_checkpoint(&current)?;
    let target = predicate
        .cache
        .get(&mask_hash(&current))
        .and_then(|(_, target)| target.clone())
        .ok_or("best verified target missing")?;
    let retained_events = predicate.events.kept(&current).len();
    Ok(Reduction {
        alive: current,
        checkpoint,
        target,
        log: predicate.log,
        closure_1_minimal: certified && stop_reason.is_none(),
        retained_events,
        predicate_calls: budget.calls,
        stop_reason,
    })
}

fn evaluate_batch(
    predicate: &mut ReductionPredicate<'_>,
    budget: &mut Budget,
    batch: &[AliveMask],
) -> Result<Option<AliveMask>, String> {
    let remaining = budget.limits.max_calls.saturating_sub(budget.calls);
    let count = batch
        .len()
        .min(usize::try_from(remaining).unwrap_or(usize::MAX));
    if count == 0 || budget.reason().is_some() {
        return Ok(None);
    }
    let masks = batch.get(..count).ok_or("invalid batch")?;
    let results = predicate.batch(masks)?;
    budget.calls = budget
        .calls
        .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    let best = results
        .iter()
        .enumerate()
        .filter(|(_, accepted)| **accepted)
        .min_by_key(|(index, _)| {
            predicate
                .events
                .kept(masks.get(*index).unwrap_or(&predicate.initial))
                .len()
        })
        .and_then(|(index, _)| masks.get(index))
        .cloned();
    Ok(best)
}
fn remove_closure(
    current: &AliveMask,
    graph: &UnitGraph,
    roots: &[usize],
    pinned: &AliveMask,
) -> Option<AliveMask> {
    let closure = graph.closure(roots);
    if closure.iter().any(|unit| pinned.contains(*unit)) {
        return None;
    }
    let mut mask = current.clone();
    for unit in closure {
        mask.set(unit, false);
    }
    if mask == *current { None } else { Some(mask) }
}
/// Pin the target's local fills and the evidence/support windows needed by them.
pub fn pinned_units(store: &TraceStore, target: &Finding) -> Result<AliveMask, String> {
    let (window, _, _) = window(store, target, None)?;
    pin_window(store, target, &window)
}
fn pin_window(
    store: &TraceStore,
    target: &Finding,
    window: &AliveMask,
) -> Result<AliveMask, String> {
    let key = target.instance_key.as_ref().ok_or("missing key")?;
    let anchor = store
        .spine
        .seq
        .binary_search(&key.anchor_claim_original_seq)
        .map_err(|_| "missing anchor")?;
    let mut pinned =
        AliveMask::empty(store.units.len()).with_range(window.bounds().0, window.bounds().1);
    pinned.set(
        *store.spine.unit.get(anchor).ok_or("missing anchor unit")? as usize,
        true,
    );
    if key.invariant_id != Invariant::I3
        && let Some(previous) = target
            .window
            .start_seq
            .and_then(|seq| store.spine.seq.binary_search(&seq).ok())
        && window.contains_event(store, previous)
    {
        pinned.set(
            *store
                .spine
                .unit
                .get(previous)
                .ok_or("invalid predecessor")? as usize,
            true,
        );
    }
    let events: Vec<_> = (0..store.len())
        .filter(|event| window.contains_event(store, *event))
        .collect();
    for &event in &events {
        let seq = *store.spine.seq.get(event).ok_or("invalid seq")?;
        if seq >= key.anchor_claim_original_seq
            || target.window.start_seq.is_some_and(|start| seq <= start)
            || store.spine.kind.get(event) != Some(&Kind::Filled)
        {
            continue;
        }
        let relevant = if key.invariant_id == Invariant::I1 {
            true
        } else if key.invariant_id == Invariant::I2 {
            store
                .spine
                .payload_idx
                .get(event)
                .and_then(|fill| store.fills.order.get(*fill as usize))
                .and_then(|order| store.orders.instrument.get(*order as usize))
                .and_then(|instrument| store.header.instruments.get(*instrument as usize))
                .is_some_and(|instrument| instrument.id == key.scope)
        } else {
            false
        };
        if relevant {
            pinned.set(
                *store.spine.unit.get(event).ok_or("invalid unit")? as usize,
                true,
            );
        }
    }
    loop {
        let before = pinned.clone();
        let mut cash_fills = Vec::new();
        let mut position_fills = vec![Vec::new(); store.header.instruments.len()];
        for &event in &events {
            let unit = *store.spine.unit.get(event).ok_or("invalid unit")? as usize;
            let payload = *store
                .spine
                .payload_idx
                .get(event)
                .ok_or("invalid payload")? as usize;
            match store.spine.kind.get(event) {
                Some(Kind::Filled) => {
                    cash_fills.push(unit);
                    let instrument = store
                        .fills
                        .order
                        .get(payload)
                        .and_then(|order| store.orders.instrument.get(*order as usize))
                        .copied()
                        .ok_or("invalid instrument")? as usize;
                    position_fills
                        .get_mut(instrument)
                        .ok_or("invalid instrument")?
                        .push(unit);
                }
                Some(Kind::CashClaim) => {
                    if cash_fills.iter().any(|fill| pinned.contains(*fill)) {
                        pinned.set(unit, true);
                    }
                    if pinned.contains(unit) {
                        for fill in &cash_fills {
                            pinned.set(*fill, true);
                        }
                    }
                    cash_fills.clear();
                }
                Some(Kind::PositionClaim) => {
                    let instrument = *store
                        .position_claims
                        .instrument
                        .get(payload)
                        .ok_or("invalid claim")? as usize;
                    let fills = position_fills
                        .get_mut(instrument)
                        .ok_or("invalid instrument")?;
                    if fills.iter().any(|fill| pinned.contains(*fill)) {
                        pinned.set(unit, true);
                    }
                    if pinned.contains(unit) {
                        for fill in fills.iter() {
                            pinned.set(*fill, true);
                        }
                    }
                    fills.clear();
                }
                _ => {}
            }
        }
        if pinned == before {
            break;
        }
    }
    Ok(pinned)
}
