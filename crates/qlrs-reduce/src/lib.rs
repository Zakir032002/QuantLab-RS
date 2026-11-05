//! Claim-boundary lifecycle reduction with bounded deterministic search.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
mod algebra;
mod cache;
mod certificate;
mod log;
pub use algebra::{ContributionTable, FastPredicate};
use cache::{Cached, HashState, lookup};
pub use certificate::{Certificate, ClosureTest, Rejection, UnitIdentity, rle};
pub use log::{StepLog, StepOutcome};
use qlrs_check::{Checker, Finding, InstanceKey, Invariant, Report};
use qlrs_ledger::Ledger;
use qlrs_schema::{AliveMask, Kind, Side, Status, SyntheticCheckpoint, TraceStore};
use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    time::{Duration, Instant},
};

/// Dependency-to-dependent CSR graph, with reverse prerequisite edges.
#[derive(Debug, Clone)]
pub struct UnitGraph {
    /// Original unit IDs for compact CSR rows, sorted.
    pub unit_ids: Vec<u32>,
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
        for event in event_range(store, window) {
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
        let mut unit_ids: Vec<_> = event_range(store, window)
            .filter(|event| window.contains_event(store, *event))
            .filter_map(|event| store.spine.unit.get(event).copied())
            .collect();
        unit_ids.sort_unstable();
        unit_ids.dedup();
        let local: Result<Vec<_>, String> = edges
            .into_iter()
            .map(|(from, to)| {
                Ok((
                    qlrs_schema::checked_index(
                        unit_ids
                            .binary_search(&from)
                            .map_err(|_| "missing dependency")?,
                    )
                    .map_err(|_| "too many units")?,
                    qlrs_schema::checked_index(
                        unit_ids
                            .binary_search(&to)
                            .map_err(|_| "missing dependent")?,
                    )
                    .map_err(|_| "too many units")?,
                ))
            })
            .collect();
        let local = local?;
        let (offsets, targets) = csr(unit_ids.len(), &local)?;
        let mut reverse: Vec<_> = local.into_iter().map(|(a, b)| (b, a)).collect();
        reverse.sort_unstable();
        let (parent_offsets, parents) = csr(unit_ids.len(), &reverse)?;
        Ok(Self {
            unit_ids,
            offsets,
            targets,
            parent_offsets,
            parents,
        })
    }
    fn walk(&self, offsets: &[u32], targets: &[u32], roots: &[usize]) -> Vec<usize> {
        let local: Vec<_> = roots
            .iter()
            .filter_map(|unit| {
                qlrs_schema::checked_index(*unit)
                    .ok()
                    .and_then(|id| self.unit_ids.binary_search(&id).ok())
            })
            .collect();
        walk(offsets, targets, &local)
            .into_iter()
            .filter_map(|row| self.unit_ids.get(row).map(|id| *id as usize))
            .collect()
    }
    /// Sorted transitive removal closure, including the roots.
    #[must_use]
    pub fn closure(&self, roots: &[usize]) -> Vec<usize> {
        self.walk(&self.offsets, &self.targets, roots)
    }
    /// Sorted transitive prerequisites, including the roots.
    #[must_use]
    pub fn prerequisites(&self, roots: &[usize]) -> Vec<usize> {
        self.walk(&self.parent_offsets, &self.parents, roots)
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
fn event_range(store: &TraceStore, mask: &AliveMask) -> std::ops::Range<usize> {
    let (first, last) = mask.bounds();
    let start = first.map_or(0, |seq| {
        store.spine.seq.partition_point(|value| *value < seq)
    });
    let end = last.map_or(store.len(), |seq| {
        store.spine.seq.partition_point(|value| *value <= seq)
    });
    start..end
}
struct UnitEvents {
    unit_ids: Vec<u32>,
    offsets: Vec<u32>,
    events: Vec<u32>,
}
impl UnitEvents {
    fn new(store: &TraceStore, window: &AliveMask) -> Result<Self, String> {
        let mut pairs = Vec::new();
        for event in event_range(store, window) {
            if window.contains_event(store, event) {
                pairs.push((
                    *store.spine.unit.get(event).ok_or("invalid spine")?,
                    qlrs_schema::checked_index(event).map_err(|_| "too many events")?,
                ));
            }
        }
        pairs.sort_unstable();
        let mut unit_ids = Vec::new();
        let mut offsets = Vec::new();
        let mut events = Vec::with_capacity(pairs.len());
        for (unit, event) in pairs {
            if unit_ids.last() != Some(&unit) {
                unit_ids.push(unit);
                offsets
                    .push(qlrs_schema::checked_index(events.len()).map_err(|_| "too many events")?);
            }
            events.push(event);
        }
        offsets.push(qlrs_schema::checked_index(events.len()).map_err(|_| "too many events")?);
        Ok(Self {
            unit_ids,
            offsets,
            events,
        })
    }
    fn kept(&self, alive: &AliveMask) -> Vec<usize> {
        let mut events = Vec::new();
        for (row, unit) in self.unit_ids.iter().enumerate() {
            if !alive.contains(*unit as usize) {
                continue;
            }
            let start = self.offsets.get(row).copied().unwrap_or(0) as usize;
            let end = self
                .offsets
                .get(row.saturating_add(1))
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
    let end = anchor;
    let end_seq = *store.spine.seq.get(end).ok_or("missing end")?;
    let mut mask = AliveMask::empty(store.units.len()).with_range(cut, Some(end_seq));
    for event in first..=end {
        let kind = store.spine.kind.get(event).ok_or("invalid event")?;
        let wanted_claim = match kind {
            Kind::CashClaim => key.invariant_id == Invariant::I1,
            Kind::PositionClaim => {
                key.invariant_id == Invariant::I2
                    && store
                        .spine
                        .payload_idx
                        .get(event)
                        .and_then(|payload| store.position_claims.instrument.get(*payload as usize))
                        .and_then(|instrument| store.header.instruments.get(*instrument as usize))
                        .is_some_and(|instrument| instrument.id == key.scope)
            }
            _ => true,
        };
        if !wanted_claim {
            continue;
        }
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
    contributions: ContributionTable,
    cache: HashMap<[u64; 2], Vec<Cached<CandidateResult>>, foldhash::fast::RandomState>,
    hash_state: HashState,
    hash_context: [u64; 2],
    full_calls: u64,
    fast_probes: u64,
    commits: Vec<Commit>,
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
        let contributions = ContributionTable::new(store, &initial, &target)?;
        let mut context = blake3::Hasher::new();
        context.update(b"qlrs-mask/v2");
        context.update(source_digest.as_bytes());
        context.update(
            &u64::try_from(initial.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        for bound in [initial.bounds().0, initial.bounds().1] {
            context.update(&[u8::from(bound.is_some())]);
            context.update(&bound.unwrap_or(0).to_le_bytes());
        }
        let bytes = context.finalize();
        let hash_context = [
            u64::from_le_bytes(
                bytes
                    .as_bytes()
                    .get(..8)
                    .ok_or("invalid hash")?
                    .try_into()
                    .map_err(|_| "invalid hash")?,
            ),
            u64::from_le_bytes(
                bytes
                    .as_bytes()
                    .get(8..16)
                    .ok_or("invalid hash")?
                    .try_into()
                    .map_err(|_| "invalid hash")?,
            ),
        ];
        Ok(Self {
            store,
            target,
            original: original.keys().cloned().collect(),
            checkpoint,
            initial,
            events,
            contributions,
            cache: HashMap::default(),
            hash_state: HashState::default(),
            hash_context,
            full_calls: 0,
            fast_probes: 0,
            commits: Vec::new(),
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
        Ok(Checker::check_witness_selected(
            self.store,
            mask,
            cp.as_ref(),
            &self.target,
            &events,
        ))
    }
    fn classify(&self, mask: &AliveMask) -> Result<CandidateResult, String> {
        let report = self.report(mask)?;
        if self.admits(&report) {
            let finding = report
                .findings
                .into_iter()
                .find(|finding| finding.instance_key.as_ref() == Some(&self.target));
            return Ok(CandidateResult {
                accepted: true,
                finding,
                rejection: None,
            });
        }
        let outcome = report
            .keys()
            .find(|key| key.invariant_id == Invariant::I3 && **key != self.target)
            .map(|key| Rejection::LifecycleInvalid {
                sub_code: key.sub_code.clone(),
            })
            .or_else(|| {
                report
                    .keys()
                    .find(|key| !self.original.contains(*key))
                    .map(|key| {
                        if key.invariant_id == self.target.invariant_id
                            && key.scope == self.target.scope
                            && key.anchor_claim_original_seq
                                == self.target.anchor_claim_original_seq
                        {
                            Rejection::ResidualChanged {
                                new_residual: key.residual_raw.clone(),
                            }
                        } else {
                            Rejection::NewViolation { key: key.clone() }
                        }
                    })
            })
            .or_else(|| {
                self.contributions
                    .residual(mask)
                    .filter(|residual| residual.to_string() != self.target.residual_raw)
                    .map(|residual| Rejection::ResidualChanged {
                        new_residual: residual.to_string(),
                    })
            })
            .unwrap_or(Rejection::KeyMissing);
        Ok(CandidateResult {
            accepted: false,
            finding: None,
            rejection: Some(outcome),
        })
    }
    fn record_commit(&mut self, mask: &AliveMask) {
        let units = self.canonical(mask);
        if self
            .commits
            .last()
            .is_some_and(|commit| commit.units == units)
        {
            return;
        }
        self.commits.push(Commit {
            units,
            events: self.events.kept(mask).len(),
            full: self.full_calls,
            fast: self.fast_probes,
        });
    }
    fn canonical(&self, mask: &AliveMask) -> Vec<u32> {
        self.events
            .unit_ids
            .iter()
            .copied()
            .filter(|unit| mask.contains(*unit as usize))
            .collect()
    }
    fn cached(&mut self, mask: &AliveMask) -> Option<CandidateResult> {
        let units = self.canonical(mask);
        let hash = self.hash_state.update(&units);
        lookup(self.cache.get(&hash), &units).cloned()
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
    fn validate_candidates(&self, masks: &[AliveMask]) -> Result<(), String> {
        if masks
            .iter()
            .any(|mask| mask.bounds() != self.initial.bounds())
        {
            return Err("candidate changes the fixed observation window".into());
        }
        Ok(())
    }
    fn batch(&mut self, masks: &[AliveMask]) -> Result<Vec<bool>, String> {
        self.batch_mode(masks, false)
    }
    fn batch_mode(&mut self, masks: &[AliveMask], speculative: bool) -> Result<Vec<bool>, String> {
        self.validate_candidates(masks)?;
        let canonical: Vec<_> = masks.iter().map(|mask| self.canonical(mask)).collect();
        let hashes: Vec<_> = canonical
            .iter()
            .map(|units| self.hash_state.update(units))
            .collect();
        let known: Vec<_> = hashes
            .iter()
            .zip(&canonical)
            .map(|(hash, units)| lookup(self.cache.get(hash), units).cloned())
            .collect();
        let canceled: Vec<_> = masks.iter().map(|_| AtomicBool::new(false)).collect();
        let results: Vec<Result<Option<CandidateResult>, String>> = masks
            .par_iter()
            .zip(&known)
            .enumerate()
            .map(|(index, (mask, cached))| {
                if speculative
                    && canceled
                        .get(index)
                        .is_some_and(|flag| flag.load(Ordering::Relaxed))
                {
                    return Ok(None);
                }
                let value = if let Some(value) = cached {
                    value.clone()
                } else {
                    self.classify(mask)?
                };
                if speculative && value.accepted {
                    for flag in canceled.get(index.saturating_add(1)..).unwrap_or_default() {
                        flag.store(true, Ordering::Relaxed);
                    }
                }
                Ok(Some(value))
            })
            .collect();
        let values = results;
        let cutoff = if speculative {
            values
                .iter()
                .position(|value| {
                    value
                        .as_ref()
                        .ok()
                        .and_then(|value| value.as_ref())
                        .is_some_and(|value| value.accepted)
                })
                .map_or(masks.len(), |index| index.saturating_add(1))
        } else {
            masks.len()
        };
        // Deterministic conservative reservation, independent of which higher
        // probes finished before cancellation. Discard their outcomes and cache
        // effects; only the prefix through the lowest success is observable.
        self.full_calls = self.full_calls.saturating_add(
            u64::try_from(known.iter().filter(|cached| cached.is_none()).count())
                .unwrap_or(u64::MAX),
        );
        let mut out = Vec::with_capacity(masks.len());
        for (index, ((((mask, units), hash), cached), value)) in masks
            .iter()
            .zip(canonical)
            .zip(hashes)
            .zip(known)
            .zip(values)
            .enumerate()
        {
            if index >= cutoff {
                out.push(false);
                continue;
            }
            let value = value?.ok_or("a lower speculative probe was incorrectly canceled")?;
            let accepted = value.accepted;
            if cached.is_none() {
                self.cache
                    .entry(hash)
                    .or_default()
                    .push(Cached { units, value });
            }
            self.log.push(log_entry(
                contextual_hash(hash, self.hash_context),
                self.store.units.len(),
                mask,
                accepted,
                cached.is_some(),
            ));
            out.push(accepted);
        }
        Ok(out)
    }
}
struct Commit {
    units: Vec<u32>,
    events: usize,
    full: u64,
    fast: u64,
}
#[derive(Clone)]
struct CandidateResult {
    accepted: bool,
    finding: Option<Finding>,
    rejection: Option<Rejection>,
}

impl Predicate for ReductionPredicate<'_> {
    fn test(&mut self, alive: &AliveMask) -> Result<Outcome, String> {
        let mask = self.candidate_mask(alive);
        self.validate_candidates(std::slice::from_ref(&mask))?;
        let units = self.canonical(&mask);
        let hash = contextual_hash(self.hash_state.update(&units), self.hash_context);
        let cached = self.cached(&mask).is_some();
        let accepted = self
            .batch(std::slice::from_ref(&mask))?
            .first()
            .copied()
            .unwrap_or(false);
        Ok(Outcome {
            accepted,
            digest: hash,
            cache_hit: cached,
        })
    }
}
fn contextual_hash(hash: [u64; 2], context: [u64; 2]) -> String {
    format!(
        "{:016x}{:016x}",
        hash.first().copied().unwrap_or(0) ^ context.first().copied().unwrap_or(0),
        hash.get(1).copied().unwrap_or(0) ^ context.get(1).copied().unwrap_or(0)
    )
}
fn log_entry(hash: String, units: usize, mask: &AliveMask, result: bool, cached: bool) -> LogEntry {
    LogEntry {
        candidate_digest: hash,
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
    /// Portable complete/partial closure proof.
    pub certificate: Certificate,
    /// Reserved full-check slots; canceled speculation is conservatively included.
    pub full_calls: u64,
    /// Checked numerical prefilter queries.
    pub fast_probes: u64,
    /// Versioned compact wire log, separate from legacy candidate telemetry.
    pub steps: Vec<StepLog>,
}
/// Search strategy; SBR is the default, ddmin remains a reference implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    /// Exact algebraic seed followed by binary prefix search over closures.
    #[default]
    Sbr,
    /// Deterministic ddmin over dependency-closed removals.
    DdminClosures,
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
    reduce_with_options(
        store,
        digest,
        checkpoint,
        target_seq,
        limits,
        Algorithm::Sbr,
    )
}
/// Select the reference search without changing witness admission semantics.
pub fn reduce_with_options(
    store: &TraceStore,
    digest: &str,
    checkpoint: Option<&SyntheticCheckpoint>,
    target_seq: Option<u64>,
    limits: ReductionLimits,
    algorithm: Algorithm,
) -> Result<Reduction, String> {
    let mut budget = Budget {
        start: Instant::now(),
        limits,
        calls: 0,
    };
    let all = AliveMask::all(store.units.len());
    let original = Checker::check(store, &all, checkpoint, true);
    if original.status != Status::Violation
        || original.findings.iter().any(|finding| {
            finding.status == Status::Unsupported
                || (finding.status == Status::Inconclusive
                    && !finding
                        .reason
                        .as_deref()
                        .is_some_and(|reason| reason.starts_with("missing-later-")))
        })
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
    let mut predicate = ReductionPredicate::new(store, key.clone(), &original, checkpoint, digest)?;
    let mut current = predicate.initial.clone();
    // This first candidate preserves the complete original local window; it is
    // verified before becoming the fallback for any later exhausted search.
    let verified = predicate
        .batch(std::slice::from_ref(&current))?
        .first()
        .copied()
        .unwrap_or(false);
    if budget.reason().is_none() {
        budget.calls = budget.calls.saturating_add(1);
    }
    if !verified {
        if budget.reason().is_some() {
            return Err(
                "budget exhausted before a lifecycle-valid target witness was verified".into(),
            );
        }
        let pins = pin_window(store, &target, &current)?;
        let graph =
            UnitGraph::build_with_checkpoint(store, predicate.checkpoint.as_ref(), &current)?;
        let mut roots = predicate.contributions.seed_roots();
        roots.extend(pins.iter());
        let mut seed =
            AliveMask::empty(store.units.len()).with_range(current.bounds().0, current.bounds().1);
        for unit in graph.prerequisites(&roots) {
            if current.contains(unit) {
                seed.set(unit, true);
            }
        }
        let accepted = predicate
            .batch(std::slice::from_ref(&seed))?
            .first()
            .copied()
            .unwrap_or(false);
        budget.calls = budget.calls.saturating_add(1);
        if !accepted {
            return Err("claim-boundary context failed exact original replay".into());
        }
        current = seed;
    }
    if key.invariant_id == Invariant::I2 && budget.reason().is_some() {
        let pins = pin_window(store, &target, &current)?;
        let graph =
            UnitGraph::build_with_checkpoint(store, predicate.checkpoint.as_ref(), &current)?;
        let mut roots = predicate.contributions.seed_roots();
        roots.extend(pins.iter());
        let mut scoped =
            AliveMask::empty(store.units.len()).with_range(current.bounds().0, current.bounds().1);
        for unit in graph.prerequisites(&roots) {
            if current.contains(unit) {
                scoped.set(unit, true);
            }
        }
        if scoped != current
            && predicate
                .batch(std::slice::from_ref(&scoped))?
                .first()
                .copied()
                .unwrap_or(false)
        {
            current = scoped;
        }
    }
    predicate.record_commit(&current);
    if budget.reason().is_some() {
        return finish_reduction(predicate, current, &budget, false, Vec::new());
    }
    let pinned = pin_window(store, &target, &current)?;
    let graph = UnitGraph::build_with_checkpoint(store, predicate.checkpoint.as_ref(), &current)?;
    let mut sliced =
        AliveMask::empty(store.units.len()).with_range(current.bounds().0, current.bounds().1);
    let mut roots = predicate.contributions.seed_roots();
    roots.extend(pinned.iter());
    for unit in graph.prerequisites(&roots) {
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
            current = sliced.clone();
            predicate.record_commit(&current);
        }
    }
    if algorithm == Algorithm::DdminClosures {
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
                            predicate.record_commit(&current);
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
    } else if current != sliced && budget.reason().is_none() {
        // The seed was rejected. Search cost-sorted prerequisite unions. Full
        // acceptance is not assumed monotone; every committed prefix is checked.
        let base = sliced;
        let mut additions: Vec<_> = current
            .iter()
            .filter(|unit| !base.contains(*unit))
            .map(|unit| {
                let closure = graph.prerequisites(&[unit]);
                let mut mask = base.clone();
                for id in &closure {
                    if current.contains(*id) {
                        mask.set(*id, true);
                    }
                }
                (predicate.events.kept(&mask).len(), unit, closure)
            })
            .collect();
        additions.sort_by_key(|(cost, unit, _)| (*cost, *unit));
        let closures: Vec<_> = additions
            .into_iter()
            .map(|(_, _, closure)| closure)
            .collect();
        let prefix_values = predicate.contributions.closure_prefixes(&closures, &base);
        let mut low = 0_usize;
        let mut high = closures.len();
        while low < high && budget.reason().is_none() {
            let width = high.saturating_sub(low);
            let mut points: Vec<_> = (1_usize..=8)
                .map(|part| low.saturating_add(width.saturating_mul(part).div_ceil(8)))
                .collect();
            points.sort_unstable();
            points.dedup();
            let mut batch = Vec::new();
            let mut positions = Vec::new();
            for point in &points {
                predicate.fast_probes = predicate.fast_probes.saturating_add(1);
                if prefix_values
                    .as_ref()
                    .and_then(|values| values.get(*point))
                    .is_some_and(|residual| residual.to_string() != key.residual_raw)
                {
                    continue;
                }
                let mut candidate = base.clone();
                for closure in closures.get(..*point).unwrap_or_default() {
                    for unit in closure {
                        if current.contains(*unit) {
                            candidate.set(*unit, true);
                        }
                    }
                }
                positions.push(*point);
                batch.push(candidate);
            }
            if let Some(best) = evaluate_batch(&mut predicate, &mut budget, &batch)? {
                let index = batch
                    .iter()
                    .position(|mask| *mask == best)
                    .ok_or("missing winning prefix")?;
                let point = *positions.get(index).ok_or("missing prefix position")?;
                current = best;
                predicate.record_commit(&current);
                if point == 0 {
                    break;
                }
                high = point.saturating_sub(1);
            } else {
                break;
            }
            // Search a smaller prefix only; a non-monotone predicate may contain
            // earlier successes, which the final closure pass can still discover.
            low = low.min(high);
        }
    }
    // On small financial windows, enumerate numerical subsets in Gray order.
    // O(1) updates avoid checker calls for changed residuals; try admissible
    // subsets by event cost. This also removes balanced buy/sell collections
    // which a single-closure certificate alone cannot discover.
    if algorithm == Algorithm::Sbr && budget.reason().is_none() && key.invariant_id != Invariant::I3
    {
        let units: Vec<_> = current
            .iter()
            .filter(|unit| !pinned.contains(*unit))
            .collect();
        if units.len() <= 18 {
            let mut base = current.clone();
            for unit in &units {
                base.set(*unit, false);
            }
            if let Some(base_residual) = predicate.contributions.residual(&base) {
                let mut values = Vec::new();
                let mut costs = Vec::new();
                for unit in &units {
                    let value = qlrs_schema::checked_index(*unit)
                        .ok()
                        .and_then(|id| predicate.contributions.units.binary_search(&id).ok())
                        .and_then(|row| predicate.contributions.values.get(row))
                        .copied()
                        .ok_or("missing unit contribution")?;
                    values.push(value);
                    let mut single = base.clone();
                    single.set(*unit, true);
                    costs.push(
                        predicate
                            .events
                            .kept(&single)
                            .len()
                            .saturating_sub(predicate.events.kept(&base).len()),
                    );
                }
                let wanted = key
                    .residual_raw
                    .parse::<i128>()
                    .map_err(|e| e.to_string())?;
                let ceiling = predicate.events.kept(&current).len();
                let base_cost = predicate.events.kept(&base).len();
                let mut residual = base_residual;
                let mut cost = base_cost;
                let mut previous = 0_u64;
                let mut candidates = Vec::new();
                for index in 0..(1_u64 << units.len()) {
                    if budget.reason().is_some() {
                        break;
                    }
                    let gray = index ^ (index >> 1);
                    let difference = gray ^ previous;
                    if difference != 0 {
                        let row = difference.trailing_zeros() as usize;
                        let value = *values.get(row).ok_or("invalid Gray contribution")?;
                        let delta = *costs.get(row).ok_or("invalid Gray cost")?;
                        if gray & difference == 0 {
                            let Some(next) = residual.checked_add(value) else {
                                break;
                            };
                            residual = next;
                            cost = cost.checked_sub(delta).ok_or("invalid event cost")?;
                        } else {
                            let Some(next) = residual.checked_sub(value) else {
                                break;
                            };
                            residual = next;
                            cost = cost.checked_add(delta).ok_or("event count overflow")?;
                        }
                    }
                    previous = gray;
                    predicate.fast_probes = predicate.fast_probes.saturating_add(1);
                    if residual == wanted && cost < ceiling {
                        candidates.push((cost, gray));
                    }
                }
                candidates.sort_unstable();
                for (_, bits) in candidates {
                    if budget.reason().is_some() {
                        break;
                    }
                    let mut candidate = base.clone();
                    for (row, unit) in units.iter().enumerate() {
                        if bits & (1_u64 << row) != 0 {
                            candidate.set(*unit, true);
                        }
                    }
                    if graph
                        .prerequisites(&candidate.iter().collect::<Vec<_>>())
                        .iter()
                        .any(|unit| !candidate.contains(*unit))
                    {
                        continue;
                    }
                    if let Some(best) = evaluate_batch(&mut predicate, &mut budget, &[candidate])? {
                        current = best;
                        predicate.record_commit(&current);
                        break;
                    }
                }
            }
        }
    }
    let mut certified = false;
    let mut entries = Vec::new();
    let max_passes = current.count().saturating_add(1);
    for _ in 0..max_passes {
        if budget.reason().is_some() {
            break;
        }
        entries.clear();
        let cp = predicate.candidate_checkpoint(&current)?;
        let graph = UnitGraph::build_with_checkpoint(store, cp.as_ref(), &current)?;
        let mut improved = false;
        for unit in current.iter().collect::<Vec<_>>() {
            if pinned.contains(unit) {
                continue;
            }
            if budget.reason().is_some() {
                break;
            }
            if let Some(candidate) = remove_closure(&current, &graph, &[unit], &pinned) {
                // Numerical rejection is a prefilter only. The certificate records
                // the full oracle's actual rejection, independently recheckable.
                predicate.fast_probes = predicate.fast_probes.saturating_add(1);
                let _residual = predicate.contributions.residual(&candidate);
                let accepted = predicate
                    .batch(std::slice::from_ref(&candidate))?
                    .first()
                    .copied()
                    .unwrap_or(false);
                budget.calls = budget.calls.saturating_add(1);
                if accepted {
                    current = candidate;
                    predicate.record_commit(&current);
                    improved = true;
                    break;
                }
                let rejection = predicate
                    .cached(&candidate)
                    .and_then(|result| result.rejection)
                    .ok_or("missing rejection")?;
                let removed: Vec<_> = graph
                    .closure(&[unit])
                    .into_iter()
                    .filter(|id| current.contains(*id))
                    .collect();
                entries.push(ClosureTest {
                    root: qlrs_schema::checked_index(unit).map_err(|_| "too many units")?,
                    removed_closure: rle(&removed),
                    outcome: rejection,
                    evaluator: "full".into(),
                });
            }
        }
        if !improved && budget.reason().is_none() {
            certified = true;
            break;
        }
        if improved {
            entries.clear();
        }
    }
    finish_reduction(predicate, current, &budget, certified, entries)
}

fn finish_reduction(
    mut predicate: ReductionPredicate<'_>,
    current: AliveMask,
    budget: &Budget,
    certified: bool,
    entries: Vec<ClosureTest>,
) -> Result<Reduction, String> {
    let stop_reason = budget.reason();
    let checkpoint = predicate.candidate_checkpoint(&current)?;
    let target = predicate
        .cached(&current)
        .and_then(|result| result.finding)
        .ok_or("best verified target missing")?;
    // Mandatory final independent replay, even if a cooperative budget fired.
    let replayed = predicate.classify(&current)?;
    if !replayed.accepted {
        return Err("final witness failed full checker".into());
    }
    let certificate = build_certificate(
        &predicate,
        &current,
        entries,
        certified && stop_reason.is_none(),
    )?;
    let steps = compact_steps(&predicate, &current, &certificate)?;
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
        certificate,
        full_calls: predicate.full_calls,
        fast_probes: predicate.fast_probes,
        steps,
    })
}

fn compact_steps(
    predicate: &ReductionPredicate<'_>,
    current: &AliveMask,
    certificate: &Certificate,
) -> Result<Vec<StepLog>, String> {
    let mut out = Vec::new();
    let mut previous: Vec<u32> = Vec::new();
    for commit in &predicate.commits {
        let removed: Vec<_> = previous
            .iter()
            .filter(|unit| commit.units.binary_search(unit).is_err())
            .map(|id| *id as usize)
            .collect();
        out.push(StepLog {
            schema: "qlrs-reduce-log/v1".into(),
            step: u64::try_from(out.len()).map_err(|_| "too many steps")?,
            phase: if out.is_empty() { "input" } else { "search" }.into(),
            removed: rle(&removed),
            kept_units: commit.units.len(),
            kept_events: commit.events,
            calls_full: commit.full,
            probes_fast: commit.fast,
            outcome: StepOutcome::Accepted,
        });
        previous.clone_from(&commit.units);
    }
    for entry in &certificate.entries {
        let mut candidate = current.clone();
        for range in &entry.removed_closure {
            let first = *range.first().ok_or("invalid RLE")?;
            let last = *range.get(1).ok_or("invalid RLE")?;
            for unit in first..=last {
                candidate.set(unit as usize, false);
            }
        }
        out.push(StepLog {
            schema: "qlrs-reduce-log/v1".into(),
            step: u64::try_from(out.len()).map_err(|_| "too many steps")?,
            phase: "certificate".into(),
            removed: entry.removed_closure.clone(),
            kept_units: candidate.count(),
            kept_events: predicate.events.kept(&candidate).len(),
            calls_full: predicate.full_calls,
            probes_fast: predicate.fast_probes,
            outcome: StepOutcome::Rejected {
                reason: entry.outcome.clone(),
            },
        });
    }
    Ok(out)
}

fn build_certificate(
    predicate: &ReductionPredicate<'_>,
    current: &AliveMask,
    mut entries: Vec<ClosureTest>,
    complete: bool,
) -> Result<Certificate, String> {
    let graph =
        UnitGraph::build_with_checkpoint(predicate.store, predicate.checkpoint.as_ref(), current)?;
    let target = predicate
        .report(current)?
        .findings
        .into_iter()
        .find(|finding| finding.instance_key.as_ref() == Some(&predicate.target))
        .ok_or("missing final key")?;
    let pinned = pin_window(predicate.store, &target, current)?;
    let events = predicate.events.kept(current);
    let mut kept_units = Vec::new();
    for unit in current.iter() {
        let first = events
            .iter()
            .find(|event| {
                predicate
                    .store
                    .spine
                    .unit
                    .get(**event)
                    .is_some_and(|id| *id as usize == unit)
            })
            .and_then(|event| predicate.store.spine.seq.get(*event))
            .copied()
            .ok_or("empty kept unit")?;
        kept_units.push(UnitIdentity {
            id: qlrs_schema::checked_index(unit).map_err(|_| "too many units")?,
            first_original_seq: first,
        });
    }
    let candidates_tested =
        u64::try_from(entries.len()).map_err(|_| "too many certificate entries")?;
    for unit in current.iter() {
        let closure: Vec<_> = graph
            .closure(&[unit])
            .into_iter()
            .filter(|id| current.contains(*id))
            .collect();
        if closure.iter().any(|id| pinned.contains(*id)) {
            entries.push(ClosureTest {
                root: qlrs_schema::checked_index(unit).map_err(|_| "too many units")?,
                removed_closure: rle(&closure),
                outcome: Rejection::KeyMissing,
                evaluator: "fast".into(),
            });
        }
    }
    entries.sort_by_key(|entry| entry.root);
    Ok(Certificate {
        schema: "qlrs-closure-certificate/v1".into(),
        predicate_version: "qlrs-predicate/v1".into(),
        target: predicate.target.clone(),
        original_keys: predicate.original.iter().cloned().collect(),
        graph_digest: graph_commitment(&graph, &kept_units)?,
        kept_units,
        pinned: pinned
            .iter()
            .filter_map(|id| qlrs_schema::checked_index(id).ok())
            .collect(),
        entries,
        candidates_tested,
        vacuous: candidates_tested == 0,
        complete,
    })
}
fn graph_commitment(graph: &UnitGraph, identities: &[UnitIdentity]) -> Result<String, String> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"qlrs-graph/v1");
    let mut identities_sorted: Vec<_> = identities.iter().collect();
    identities_sorted.sort_by_key(|identity| identity.first_original_seq);
    for identity in identities_sorted {
        hash.update(&identity.first_original_seq.to_le_bytes());
        let row = graph
            .unit_ids
            .binary_search(&identity.id)
            .map_err(|_| "unknown graph row")?;
        let start = graph
            .offsets
            .get(row)
            .copied()
            .ok_or("missing graph offset")? as usize;
        let end = graph
            .offsets
            .get(row.saturating_add(1))
            .copied()
            .ok_or("missing graph offset")? as usize;
        let mut dependents: Vec<_> = graph
            .targets
            .get(start..end)
            .ok_or("invalid graph row")?
            .iter()
            .map(|target| {
                let id = *graph
                    .unit_ids
                    .get(*target as usize)
                    .ok_or("invalid graph target")?;
                identities
                    .iter()
                    .find(|identity| identity.id == id)
                    .map(|identity| identity.first_original_seq)
                    .ok_or("unknown graph unit")
            })
            .collect::<Result<_, _>>()?;
        dependents.sort_unstable();
        for seq in dependents {
            hash.update(&seq.to_le_bytes());
        }
        hash.update(&u64::MAX.to_le_bytes());
    }
    Ok(hash.finalize().to_hex().to_string())
}
/// Rebuild the graph and replay every listed removal on a standalone witness.
/// Original unit IDs are translated through immutable source sequence IDs.
pub fn recheck_certificate(
    store: &TraceStore,
    checkpoint: Option<&SyntheticCheckpoint>,
    certificate: &Certificate,
) -> Result<(), String> {
    if certificate.schema != "qlrs-closure-certificate/v1"
        || certificate.predicate_version != "qlrs-predicate/v1"
        || certificate.vacuous != (certificate.candidates_tested == 0)
    {
        return Err("invalid certificate metadata".into());
    }
    let mut mapping = HashMap::new();
    let all = AliveMask::all(store.units.len());
    for identity in &certificate.kept_units {
        let event = store
            .spine
            .seq
            .binary_search(&identity.first_original_seq)
            .map_err(|_| "missing source unit")?;
        let unit = *store.spine.unit.get(event).ok_or("missing unit")? as usize;
        if mapping.insert(identity.id, unit).is_some() {
            return Err("duplicate certificate unit".into());
        }
    }
    if mapping.len() != store.units.len()
        || mapping.values().copied().collect::<BTreeSet<_>>().len() != store.units.len()
    {
        return Err("certificate unit coverage mismatch".into());
    }
    let graph = UnitGraph::build_with_checkpoint(store, checkpoint, &all)?;
    let mut local = certificate.kept_units.clone();
    for identity in &mut local {
        identity.id =
            qlrs_schema::checked_index(*mapping.get(&identity.id).ok_or("missing mapping")?)
                .map_err(|_| "too many units")?;
    }
    if graph_commitment(&graph, &local)? != certificate.graph_digest {
        return Err("certificate graph digest mismatch".into());
    }
    let original = Checker::check_witness(store, &all, checkpoint, &certificate.target);
    let mut predicate = ReductionPredicate::new(
        store,
        certificate.target.clone(),
        &original,
        checkpoint,
        &"0".repeat(64),
    )?;
    predicate.original = certificate.original_keys.iter().cloned().collect();
    if predicate.original.len() != certificate.original_keys.len()
        || certificate
            .original_keys
            .windows(2)
            .any(|pair| pair.first() >= pair.get(1))
        || !predicate.classify(&all)?.accepted
    {
        return Err("certificate baseline predicate failed".into());
    }
    let target = original
        .findings
        .iter()
        .find(|finding| finding.instance_key.as_ref() == Some(&certificate.target))
        .ok_or("target absent")?;
    let expected_pins = pin_window(store, target, &all)?;
    let mapped_pins: BTreeSet<_> = certificate
        .pinned
        .iter()
        .map(|id| mapping.get(id).copied().ok_or("unknown pinned unit"))
        .collect::<Result<_, _>>()?;
    if mapped_pins != expected_pins.iter().collect() {
        return Err("certificate pins are not minimal boundaries".into());
    }
    let mut roots = BTreeSet::new();
    let mut tested = 0_u64;
    for entry in &certificate.entries {
        if !roots.insert(entry.root) || !matches!(entry.evaluator.as_str(), "fast" | "full") {
            return Err("duplicate or invalid certificate entry".into());
        }
        let root = *mapping.get(&entry.root).ok_or("unknown certificate root")?;
        let mut removed = Vec::new();
        for range in &entry.removed_closure {
            let first = *range.first().ok_or("invalid RLE")?;
            let last = *range.get(1).ok_or("invalid RLE")?;
            if first > last
                || u64::from(last).saturating_sub(u64::from(first))
                    >= u64::try_from(mapping.len()).unwrap_or(0)
            {
                return Err("invalid RLE range".into());
            }
            for id in first..=last {
                removed.push(*mapping.get(&id).ok_or("unknown removed unit")?);
            }
        }
        let canonical: Vec<_> = graph
            .closure(&[root])
            .into_iter()
            .filter(|unit| all.contains(*unit))
            .collect();
        removed.sort_unstable();
        if removed != canonical {
            return Err("certificate closure mismatch".into());
        }
        let mut candidate = all.clone();
        for unit in removed {
            candidate.set(unit, false);
        }
        let result = predicate.classify(&candidate)?;
        if result.accepted || result.rejection.as_ref() != Some(&entry.outcome) {
            return Err("certificate rejection not reproduced".into());
        }
        let structural = canonical.iter().any(|unit| expected_pins.contains(*unit));
        if (entry.evaluator == "fast") != structural {
            return Err("invalid certificate evaluator".into());
        }
        if entry.evaluator == "full" {
            tested = tested.saturating_add(1);
        }
    }
    if tested != certificate.candidates_tested
        || (certificate.complete && roots.len() != mapping.len())
    {
        return Err("certificate is incomplete or miscounts candidates".into());
    }
    Ok(())
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
    let results = predicate.batch_mode(masks, true)?;
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
/// Pin only the target event and its observed opening boundary.
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
    Ok(pinned)
}

#[cfg(test)]
mod helper_tests {
    #![allow(clippy::unwrap_used, clippy::too_many_lines)]
    use super::*;
    use qlrs_schema::{
        Account, CashRounding, Currency, Header, Instrument, Model, NONE, Origin, Unit,
    };
    fn fixture() -> TraceStore {
        let mut store = TraceStore::new(Header {
            record_type: "header".into(),
            schema: "qlrs-trace/v0".into(),
            model: Model {
                account_type: "CASH".into(),
                base_currency: "INR".into(),
                engine_cash_rounding: CashRounding::Exact,
                commission_max_scale: Some(2),
            },
            currencies: vec![Currency {
                code: "INR".into(),
                precision: 2,
            }],
            instruments: vec![Instrument {
                id: "X".into(),
                quote_currency: "INR".into(),
                qty_precision: 3,
                px_precision: 2,
            }],
            accounts: vec![Account {
                account_id: "A".into(),
                starting_balance: "10000.00".into(),
            }],
            origin: Origin {
                producer: "helper-regression".into(),
                producer_version: "1".into(),
                generated_by_qlrs: true,
            },
        });
        let a = store.strings.intern("O0").unwrap();
        let b = store.strings.intern("O1").unwrap();
        let trade = store.strings.intern("T").unwrap();
        store.orders.id = vec![a, b];
        store.orders.instrument = vec![0, 0];
        store.orders.side = vec![Side::Buy, Side::Buy];
        store.orders.qty_raw = vec![20_000, 20_000];
        store.orders.qty_scale = vec![3, 3];
        store.orders.unit = vec![0, 1];
        store.units = vec![
            Unit {
                order: Some(0),
                first_event: 0,
            },
            Unit {
                order: Some(1),
                first_event: 2,
            },
            Unit {
                order: None,
                first_event: 5,
            },
            Unit {
                order: None,
                first_event: 6,
            },
        ];
        store.spine.seq = (1..=7).collect();
        store.spine.ts = (1..=7).collect();
        store.spine.kind = vec![
            Kind::Submitted,
            Kind::Accepted,
            Kind::Submitted,
            Kind::Accepted,
            Kind::Filled,
            Kind::CashClaim,
            Kind::PositionClaim,
        ];
        store.spine.unit = vec![0, 0, 1, 1, 0, 2, 3];
        store.spine.payload_idx = vec![0, 0, 1, 1, 0, 0, 0];
        store.fills.order = vec![0];
        store.fills.trade = vec![trade];
        store.fills.qty_raw = vec![20_000];
        store.fills.qty_scale = vec![3];
        store.fills.px_raw = vec![10_000];
        store.fills.px_scale = vec![2];
        store.fills.fee_raw = vec![1000];
        store.fills.fee_scale = vec![2];
        store.cash_claims.raw = vec![798_900];
        store.cash_claims.scale = vec![2];
        store.cash_claims.text = vec![NONE];
        store.position_claims.raw = vec![20_000];
        store.position_claims.scale = vec![3];
        store.position_claims.text = vec![NONE];
        store.position_claims.instrument = vec![0];
        Ledger::prepare(&mut store);
        store
    }
    #[test]
    fn req_reduce_014_evaluate_batch_selects_success_and_obeys_limits() {
        let store = fixture();
        let report = Checker::check(&store, &AliveMask::all(store.units.len()), None, true);
        let target = report.at_seq(6).unwrap().instance_key.clone().unwrap();
        let mut predicate =
            ReductionPredicate::new(&store, target, &report, None, &"0".repeat(64)).unwrap();
        let full = predicate.initial.clone();
        let mut small = full.clone();
        small.set(1, false);
        let mut empty = full.clone();
        for unit in full.iter() {
            empty.set(unit, false);
        }
        let batch = vec![empty, small.clone(), full];
        let mut budget = Budget {
            start: Instant::now(),
            limits: ReductionLimits {
                max_calls: 0,
                time_limit: Duration::from_secs(10),
            },
            calls: 0,
        };
        assert_eq!(
            evaluate_batch(&mut predicate, &mut budget, &batch).unwrap(),
            None
        );
        assert_eq!(budget.calls, 0);
        budget.limits.max_calls = 1;
        assert_eq!(
            evaluate_batch(&mut predicate, &mut budget, &batch).unwrap(),
            None
        );
        assert_eq!(budget.calls, 1);
        budget.calls = 0;
        budget.limits.max_calls = 2;
        assert_eq!(
            evaluate_batch(&mut predicate, &mut budget, &batch).unwrap(),
            Some(small.clone())
        );
        assert_eq!(budget.calls, 2);
        budget.calls = 0;
        budget.limits.max_calls = 100;
        budget.limits.time_limit = Duration::ZERO;
        assert_eq!(
            evaluate_batch(&mut predicate, &mut budget, &batch).unwrap(),
            None
        );
        assert_eq!(budget.calls, 0);
    }
    #[test]
    fn req_reduce_015_remove_closure_respects_dependents_pins_and_noops() {
        let edges = vec![(0, 1), (1, 2)];
        let (offsets, targets) = csr(4, &edges).unwrap();
        let (parent_offsets, parents) = csr(4, &[(1, 0), (2, 1)]).unwrap();
        let graph = UnitGraph {
            unit_ids: vec![0, 1, 2, 3],
            offsets,
            targets,
            parent_offsets,
            parents,
        };
        let current = AliveMask::all(4).with_range(Some(10), Some(20));
        let mut pinned = AliveMask::empty(4);
        pinned.set(2, true);
        assert_eq!(remove_closure(&current, &graph, &[0], &pinned), None);
        assert_eq!(remove_closure(&current, &graph, &[4], &pinned), None);
        let small = remove_closure(&current, &graph, &[3], &pinned).unwrap();
        assert_eq!(small.iter().collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(small.bounds(), current.bounds());
        assert_eq!(remove_closure(&small, &graph, &[3], &pinned), None);
        let smaller = remove_closure(&current, &graph, &[1], &AliveMask::empty(4)).unwrap();
        assert_eq!(smaller.iter().collect::<Vec<_>>(), vec![0, 3]);
    }
    #[test]
    fn req_reduce_018_graph_commitment_is_canonical_and_rle_is_exact() {
        let (offsets, targets) = csr(2, &[(0, 1)]).unwrap();
        let (parent_offsets, parents) = csr(2, &[(1, 0)]).unwrap();
        let graph = UnitGraph {
            unit_ids: vec![5, 9],
            offsets,
            targets,
            parent_offsets,
            parents,
        };
        let ids = vec![
            UnitIdentity {
                id: 5,
                first_original_seq: 10,
            },
            UnitIdentity {
                id: 9,
                first_original_seq: 20,
            },
        ];
        let mut encoded = b"qlrs-graph/v1".to_vec();
        for value in [10_u64, 20, u64::MAX, 20, u64::MAX] {
            encoded.extend_from_slice(&value.to_le_bytes());
        }
        let digest = graph_commitment(&graph, &ids).unwrap();
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, blake3::hash(&encoded).to_hex().to_string());
        let renamed = UnitGraph {
            unit_ids: vec![50, 90],
            ..graph.clone()
        };
        let renamed_ids = vec![
            UnitIdentity {
                id: 90,
                first_original_seq: 20,
            },
            UnitIdentity {
                id: 50,
                first_original_seq: 10,
            },
        ];
        assert_eq!(graph_commitment(&renamed, &renamed_ids).unwrap(), digest);
        let (offsets, targets) = csr(2, &[]).unwrap();
        let independent = UnitGraph {
            offsets,
            targets,
            ..graph
        };
        assert_ne!(graph_commitment(&independent, &ids).unwrap(), digest);
        assert_eq!(rle(&[0, 1, 2, 5, 8, 9]), vec![[0, 2], [5, 5], [8, 9]]);
        assert_eq!(rle(&[]), [] as [[u32; 2]; 0]);
    }
    #[test]
    fn req_reduce_019_speculation_commits_lowest_success_across_thread_counts() {
        let store = fixture();
        let original = Checker::check(&store, &AliveMask::all(store.units.len()), None, true);
        let key = original.at_seq(6).unwrap().instance_key.clone().unwrap();
        let mut reference = None;
        for threads in [1, 2, 8] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let (answers, log, slots) = pool.install(|| {
                let mut predicate =
                    ReductionPredicate::new(&store, key.clone(), &original, None, &"0".repeat(64))
                        .unwrap();
                let full = predicate.initial.clone();
                let mut small = full.clone();
                small.set(1, false);
                let mut empty = full.clone();
                for unit in full.iter() {
                    empty.set(unit, false);
                }
                let answers = predicate
                    .batch_mode(&[empty, full.clone(), small.clone()], true)
                    .unwrap();
                assert_eq!(answers, vec![false, true, false]);
                assert!(predicate.cached(&full).unwrap().accepted);
                assert!(predicate.cached(&small).is_none());
                assert_eq!(predicate.full_calls, 3);
                assert_eq!(predicate.log.len(), 2);
                let outcome = predicate.test(&full).unwrap();
                assert!(outcome.cache_hit);
                assert!(outcome.accepted);
                assert_eq!(outcome.digest.len(), 32);
                (answers, predicate.log, predicate.full_calls)
            });
            let result = (answers, log, slots);
            if let Some(previous) = &reference {
                assert_eq!(*previous, result);
            } else {
                reference = Some(result);
            }
        }
    }
    #[test]
    fn req_reduce_023_cache_requires_fixed_bounds_and_public_masks_are_normalized() {
        let store = fixture();
        let original = Checker::check(&store, &AliveMask::all(store.units.len()), None, true);
        let key = original.at_seq(6).unwrap().instance_key.clone().unwrap();
        let mut predicate =
            ReductionPredicate::new(&store, key, &original, None, &"0".repeat(64)).unwrap();
        let full = predicate.initial.clone();
        assert!(predicate.test(&full).unwrap().accepted);
        let clipped = full.clone().with_range(Some(5), Some(6));
        // Public candidates are normalized to the predicate's fixed window.
        assert_eq!(predicate.candidate_mask(&clipped), full);
        assert!(predicate.test(&clipped).unwrap().accepted);
        // Internal batches bypass that normalization, so enforce the cache's
        // fixed-window precondition before looking up canonical unit IDs.
        assert!(predicate.batch(std::slice::from_ref(&clipped)).is_err());
        let cached = predicate.test(&full).unwrap();
        assert!(cached.accepted);
        assert!(cached.cache_hit);
    }
}
