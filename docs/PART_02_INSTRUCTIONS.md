# QuantLab-RS Part 2: Fix the Reducer First, Then Make It Fast and Checkable

Part 2 starts by fixing the problems your review found, because the Part 1 reducer only worked on one easy trace shape. The main design change is to cut the trace at the last claim before the violation and build a labelled checkpoint that also carries open-order state. Each order is then reduced from that cut onward rather than as a whole lifecycle. Once that is in place, a seeded Binary Reduction search (Kalhauge & Palsberg) over dependency closures, driven by an exact algebraic evaluator, should be able to replace ddmin as the main search. The 171,625-call D04 run and the 60–100 s timeouts you measured should then become tens of predicate calls. That figure is an estimate, and the agent must measure it.

**TL;DR**
- **What Part 2 is:** a correctness stage (Stage A) followed by the flagship algorithm (Stage B). Stage A fixes false CONSISTENT results on uncovered fills, adds a checkpoint with open-order state (bundle schema v1), adds local I2, drops FIFO edges once the checkpoint holds the inventory, adds call and time budgets with a best-so-far result, removes the quadratic report scans, and expands the generator. Stage B adds a seeded Binary Reduction over dependency closures, a prefix-sum algebraic evaluator that is differentially tested against the full Checker, deterministic speculative parallel search, and a re-checkable closure-1-minimality certificate.
- **Why this algorithm:** ddmin needs O(n²) tests in the worst case (|c|²+3|c|), and its first-class targets assume monotonicity. Binary Reduction over closures makes O(s·log n) predicate calls. On Java bytecode, Kalhauge & Palsberg (ESEC/FSE '19) report that J-Reduce "is 12x faster and achieves more reduction than delta debugging on average". Our I1 and I2 residuals are linear in kept events, so the evaluator can answer each prefix query in O(1) from prefix sums. ProbDD and CDD trade away 1-minimality, which we need for a certificate.
- **Honest limits:** the claimed minimality is only *closure-1-minimal relative to the stated unit granularity and dependency graph*. It is not a global minimum, which is NP-complete. The witness depends on the synthetic checkpoint only for positions and open-order state; I1 is anchored on observed claims. Some fixes may change fingerprints or golden files, and those changes need your approval. Rust 1.100.0 is scheduled for 12 Nov 2026 (releases.rs) and is not out yet; stay on 1.99.1.

---

## Plain-language intro for Zakir

**What Part 2 achieves.** By the end of Part 2, `qlrs reduce` should turn any violation from the synthetic generator (all 11 violating defects, across 8 named trace shapes, including your three timeouts) into a small witness that a reviewer can check by hand. Each witness should finish within a stated time and call budget, carry a certificate showing that no single removable piece could be dropped, and stay the same inconsistency it started as. The reducer will also stop saying CONSISTENT when the evidence does not cover the end of the trace.

**What is done and what is not.** Part 1 shipped the crates, the ledger, the checker, a ddmin-based reducer and the guardrails. Your review showed the reducer only handles the generator's easy shape, has no budget, gives false reassurance on uncovered tails, produces weak I2 witnesses, has quadratic report scans, and has no mutation testing. Part 2 does not import real-engine traces, add Parquet, add a UI, add Mode E or support multiple currencies.

**What changes in the plan.**
1. CP1.9 becomes Stage A of Part 2 and is mandatory before any algorithm work.
2. The witness model changes from "whole lifecycles plus a checkpoint cut far back" to "observed anchor claim, plus a labelled checkpoint with open orders, plus lifecycle suffixes".
3. `qlrs-bundle/v0` becomes `v1`, with a v0 reader kept for compatibility.
4. Some fingerprints or golden outputs may change. Each change is a human-approval stop, not something the agent decides.
5. Part 3 inherits the generator shapes, budgets and certificate format, so its scale and fuzzing work can start from known baselines.

**Where your original spec caused weaknesses.** Several Part 1 problems trace back to the spec rather than to the agent:
- The spec made whole lifecycles the unit, kept I2 cumulative, and allowed a coverage check of "has any claim ever existed".
- Gate B used only the D03 shape, which needed 2 predicate calls, so it could not expose any of these problems.
- No budget was required.

The Part 2 spec corrects each of these explicitly.

---

## Research findings and decisions

### 1. Reduction algorithms: what the primary sources say

| Algorithm | Source | Predicate calls | Monotonicity assumption | 1-minimal? | Fit for us |
|---|---|---|---|---|---|
| ddmin | Zeller & Hildebrandt, IEEE TSE 28(2), 2002 | Worst case \|c\|²+3\|c\| (Prop. 12); best case ≤ 2·log₂\|c\| with a single cause (Prop. 13) | Assumes monotone failure for efficiency | Yes (Prop. 11) | Kept only as a fallback, run on closures |
| HDD | Misherghi & Su, ICSE 2006 | ddmin per tree level | Needs a tree structure | 1-tree-minimal (goal) | Not applicable: our traces are DAGs, not trees |
| Binary Reduction (J-Reduce) | Kalhauge & Palsberg, ESEC/FSE 2019 | O(s·log n) calls; O(s(log n·$P + n·$C + n log n)) time | Monotone P over a list of closures | Yes, when P is monotone | **Chosen core** |
| Logical reduction (lbr) | Kalhauge & Palsberg, PLDI 2021 | — | Encodes validity as propositional logic | — | Deferred; our validity is a DAG, so closures are enough |
| Perses | Sun et al., ICSE 2018 | Results "2% and 45% in size of those from DD and HDD" | Syntactic validity by construction | No explicit theorem | Shows the value of valid-by-construction candidates |
| Pardis | Gharachorlu & Sumner, FASE 2019 | "46% to 80% fewer queries" than Perses | — | Not stated | Its priority ordering idea maps to our cost sort |
| Vulcan / T-Rec / LPR | Xu et al., OOPSLA 2023; TOSEM 2025; Zhang et al., ISSTA 2024 | — | Program transformations, LLM | Goes beyond 1-minimality / none | Not applicable: we must never rewrite observed claims |
| ProbDD → CDD | Wang et al., FSE 2021; Zhang et al., ICSE 2025 | CDD: "29.91% less time and 52.03% fewer queries" than ddmin | — | **No:** "faster than ddmin, but at the cost of not guaranteeing 1-minimality" | Rejected for the certificate path |
| WDD | Zhou et al., ICSE 2025 | W_ddmin cut HDD time 51.31% | — | Inherits from ddmin | Its size weighting equals our cost function |
| Parallel ddmin | Hodován & Kiss, ICSOFT 2016 | Parallel complements-only cut runtime by 74.42%–79.78% on Chromium and WebKit tests (e.g. Chromium A: 231 s vs 904 s) | — | Yes, in the non-complement variant | Informs our speculative parallel design |
| DEMi | Scott et al., NSDI 2016 | ddmin on external events, then internal events | — | Not perfectly minimal | Closest analogue for event traces: across 10 Raft and Spark bugs, results were "within a factor of 1X to 4.6X (1.6X median)" of optimal |
| Hypothesis internal shrinking | MacIver & Donaldson, ECOOP 2020 | Reduces the choice sequence | — | Local minimum | Informs generator design (shrinkable seeds) |
| Shrinking evaluation | Keles, Miao, Lampropoulos, arXiv 2608.09935 (2026) | Measures distance to an exhaustive-search ground truth | — | — | **Adopted as the benchmark method** |

**Decision.** Use **Seeded Binary Reduction over dependency closures (SBR)**:
- The **seed** is the closure of the units whose algebraic contribution to the target instance is non-zero. It is computed exactly in O(n).
- **Binary Reduction** runs only if the seed alone does not satisfy the predicate, for example because of a new violation or a non-algebraic invariant.
- A **closure-1-minimality pass** then produces the certificate.
- **ddmin over closures** is kept behind a flag as a reference oracle for differential testing of the search. It is not the default.

**Why.**
- Kalhauge & Palsberg show that turning a dependency-laden input into a list of closures removes invalid candidates entirely. They report that Binary Reduction "gives 1.7x speedup and 1.15x smaller results" than ddmin on the same closures, and 12x faster than ddmin on raw classes.
- Their 1-minimality argument holds when P is monotone. We make P monotone by construction: claims depend on the fills in their window, and lifecycle suffixes depend on the checkpoint's open-order state. We then test that property instead of assuming it.

**Predicate-call complexity in our setting** (n = number of units in the reduction window, s = number of kept closures, k = closures in the seed):

| Phase | Worst case | Expected (estimate) |
|---|---|---|
| Algebraic seed | 0 predicate calls; O(n) arithmetic | — |
| Seed verification (full Checker) | 1 | 1 |
| Binary Reduction fallback | O(s·log₂ n) | Usually skipped; ≤ ~60 calls for n = 2,000 and s = 5 |
| Certificate pass | k | 1–10 |
| ddmin oracle (flag only) | n²+3n | — |

For comparison, your D04 measurement at 400 orders took 171,625 calls. Those calls were ddmin on whole-lifecycle units plus a far-back cut, which turned removals into slippage rejections.

### 2. Formal model of a Mode T witness

**Units.**
- A unit is either a **lifecycle suffix** (all events of one `client_order_id` with seq greater than the cut) or a single claim.
- Order state before the cut is summarised in the checkpoint's `open_orders`. This is what fixes the 12,000-event long-lived order: only its suffix inside the window is a unit.

**Dependency edges.** A kept set must be downward closed, so if a unit is kept, everything it depends on is kept.
1. Lifecycle order within a suffix, for example fill → accept. If the accept is before the cut, the dependency is satisfied by `open_orders`.
2. A claim depends on every event in its window that contributes to it. A cash claim depends on cash-affecting events since the previous cash claim on the same account. A position claim depends on fills on the same instrument since the previous position claim.
3. The FIFO sell → earlier buy edge is added **only** when checkpoint inventory plus kept earlier buys cannot cover the sell. When the checkpoint holds enough inventory, the edge is dropped. This is the D04 fix.

**Removal-stable invariants.** I1-local, I2-local and I3 are stable under removal of closed sets, because each is evaluated over a window bounded by observed claims or lifecycle state. Cumulative I2 is not stable, because earlier errors carry forward. Cumulative I2 therefore stays a reporting invariant, and witnesses use I2-local.

**Roles of claims in a witness.**
- `Anchor`: the observed claim that opens the window. Its value is used verbatim as the opening balance and is not asserted.
- `Asserted`: the target claim, whose value the witness shows to be inconsistent.
- `Evidence`: kept events inside the window.
- `Synthetic`: the labelled checkpoint, computed by the independent ledger from the full original prefix.

Observed bytes are never rewritten. The witness carries original line ranges and a BLAKE3 digest of the source.

**Validity.** A witness W for target instance key K is valid if and only if all four conditions hold:
- (a) W ∪ checkpoint replays through the ledger without errors.
- (b) Every lifecycle suffix in W is legal from the checkpoint's `open_orders` state.
- (c) The Checker run on W reports a finding whose `InstanceKey` equals K exactly. Equality covers `invariant_id`, `sub_code`, `account`, `scope`, `anchor_claim_original_seq`, `residual_raw` and `residual_scale`.
- (d) **Anti-new-violation:** every other finding in W has an `InstanceKey` that is present in the original full-trace report.

Condition (d) handles pre-existing violations without letting them be "manufactured". Carried-forward engine errors are neutralised because I1 and I2-local re-anchor at each observed claim.

**Slippage** means any accepted candidate that fails condition (c) or (d). Slippage is prevented by construction (exact key equality) and audited: the final witness is re-checked by the full Checker, never by the fast evaluator alone.

**Minimality that can honestly be claimed.** Kalhauge & Palsberg prove that minimum reduction is NP-complete by reduction from Hitting Set. We claim only **closure-1-minimality**: for every kept unit u, removing u's reverse closure (u plus everything that depends on it) makes the predicate false. The certificate records the reason for each such failure, so a reviewer can re-run each one.

**Self-evidencing witnesses.**
- **I1:** residual = (Asserted − Anchor) − Σ delta_cash(Evidence). A reviewer can add this up by hand, and the checkpoint is not needed.
- **I2-local:** expected = Anchor position + Σ fill_qty(Evidence), compared with the Asserted position.
- **I3:** the `open_orders` entry for the order at the cut, plus the suffix events, shows the illegal transition.

The checkpoint is load-bearing only for inventory (FIFO feasibility) and open-order state, and it carries `source_digest` and `cut_seq` so it can be re-derived.

**Proving the fast evaluator equals the full Checker.**
- Differential property tests over generated traces and random closed subsets compare instance-key sets for equality.
- Kani is used for the bounded arithmetic lemma that the prefix-sum residual equals the direct sum.
- Kani 0.68.0 was released on 16 Sep 2026. The tool paper (Delmas et al., "Kani: A Model Checker for Rust", ASE '26, DOI 10.1145/3832783.3834499) appeared at ASE 2026. Kani harnesses run in an optional Linux CI job, because Kani pins its own nightly toolchain.

### 3. High-performance techniques (forbid(unsafe_code))

| Technique | Where it applies | Expected effect (estimate) |
|---|---|---|
| Per-unit contribution table: i128 cash, and (instrument, i128 qty) pairs | Algebraic evaluator for I1 and I2-local | O(1) residual per prefix query once prefix sums are built |
| Prefix sums over the cost-sorted closure list | Binary Reduction prefix queries | Each probe is O(1) instead of a replay |
| CSR built from sorted, deduplicated `Vec<(u32,u32)>` edges | `UnitGraph` (replaces BTreeSet rows) | Less allocation; cache-friendly scans |
| `Vec<u64>` bitset with `count_ones` and word-wise and/or/andnot | Closure unions, kept masks | Bitwise popcount in hardware; NEON CNT on M1 |
| 128-bit tabulation/Zobrist hash, updated incrementally per toggled unit | Cache key for full-Checker results | Replaces BLAKE3 over the whole mask per candidate |
| Collision safety: store the canonical sorted kept-unit list and compare on every hit | Cache | A false hit is impossible, not merely improbable |
| Speculative p-ary search: evaluate p prefix positions in parallel and commit the lowest success | Binary Reduction on the full-Checker path | log_{p+1} n rounds; deterministic |
| `AtomicBool` cancellation of probes at higher indices once a lower index succeeds | Parallel path | Saves wasted Checker runs |
| Allocation-free `replay_into` with reused scratch buffers held per rayon thread | Full-Checker fallback | No allocator traffic on the hot path |
| Hot/cold split: delta and qty arrays separate from interned strings and source ranges | Evaluator | Fewer cache misses |
| Incremental pattern counts and run-length encoded removal logs | Checker reports, reduction log | Removes the O(n²) `window_pattern` and `order_pattern` rescans |

**Verified stack status (5 Nov 2026).** The agent must re-verify every item at implementation time.
- **Rust:** stable is 1.99.1. releases.rs lists "Stable: 1.99.1 Beta: 1.100.0 (12 November, 2026…)", so 1.100.0 has **not** shipped. Stay pinned to 1.99.1.
- **SIMD:** `std::simd` is still nightly-only. Shnatsel's survey "The state of SIMD in Rust in 2026" still lists std::simd as "(nightly)", and a portable_simd CI canary was still failing in September 2026. Since Rust 1.86, `#[target_feature]` is allowed on safe functions. Calling such a function from a context without that feature still needs `unsafe`, which `forbid(unsafe_code)` rules out. **Decision:** no explicit SIMD; rely on autovectorised `u64` loops and `count_ones`.
- **Crate versions:**
  - rayon 1.12.0 (14 Apr 2026)
  - foldhash 0.2.0 (23 Aug 2025)
  - rapidhash 4.5.1 (3 Jul 2026)
  - proptest 1.12.0 (seen in dependency-update PRs, Oct 2026)
  - cargo-mutants 27.1.0 (2 Jun 2026)
  - gungraun 0.20.0 (26 Sep 2026; Valgrind-based, Linux only, not usable on macOS arm64)
  - samply 0.13.1 (latest on docs.rs)
- **fixedbitset:** its latest is 0.5.7, released in 2024, which does not meet the actively-maintained bar. Write a ~100-line in-house `Vec<u64>` bitset instead.
- **Not verified:** bolero, cargo-fuzz, roaring, bumpalo and gxhash. The agent must check these before adding any of them, and only if needed.

### 4. Testing and verification

- **Differential tests:** fast evaluator vs. full Checker on random closed subsets (property test), and SBR vs. the ddmin-on-closures oracle on small traces.
- **Properties:**
  - validity
  - idempotence (reduce(reduce(x)) = reduce(x))
  - determinism across `RAYON_NUM_THREADS` ∈ {1, 2, 8}
  - kept size ≤ input size
  - exact `InstanceKey` preservation
  - independence from unit order (shuffled IDs give the same witness, up to ID renaming)
  - the witness replays standalone from the bundle
- **Monotonicity probe:** for random closed sets A ⊆ B where P(A) holds, assert P(B). Each counterexample is saved as a regression case and documented.
- **Ground-truth quality:** for traces with ≤ 20 units, enumerate all closed subsets exhaustively to find the true minimum, and report kept/min. This follows the Keles et al. method.
- **Mutation testing:** cargo-mutants 27.1.0 on `qlrs-reduce`, `qlrs-check` and fingerprint code, using `--in-diff` on pull requests and full runs nightly. Its documented default test timeout is "5 times the baseline test time, with a minimum of 20 seconds", tunable with `--timeout-multiplier`. Survivors are triaged into a committed table, never hidden with blanket skips.
- **Benchmark fairness:** follow J-Reduce's reporting, which gives time, predicate invocations and relative final size on a fixed corpus, with timeouts reported as best-so-far. Report per defect × shape, and never as a single aggregate.

### 5. Ecosystem check (Nov 2026)

- **NautilusTrader:** the latest release is 2.0.0rc8 (28 Oct 2026). I found no 2.0.0 final.
  - rc3 "Added canonical Rust backtest results with normalized projections, content digests, and stable ordering". This matters for Part 4 and has no effect on Part 2.
  - rc8 fixed "order fills accepting cumulative commissions beyond Money bounds". Note it for corpus triage in Part 4.
- **Competitors:** I did not do a dedicated competitor search for this part. Treat competitor status as unchecked rather than clear.

---

# PART_02_INSTRUCTIONS.md

## 0. Mission

Make `qlrs reduce` correct on all trace shapes first (Stage A), then make it fast and certifiable (Stage B).
- **Stage A:** fix every Part 1 review finding with a committed reproduction test first.
- **Stage B:** implement Seeded Binary Reduction over dependency closures with an algebraic fast evaluator, deterministic speculative parallel evaluation, slippage guards, budgets with best-so-far results, and a closure-1-minimality certificate. Pass Gate B-synthetic on D01–D11 × the named shapes.

## 1. Scope and non-goals

**In scope:**
- Fixes in `qlrs-check`, `qlrs-reduce`, `qlrs-schema` (checkpoint v1), `qlrs-bundle` (v1, with a v0 reader), `qlrs-testkit` (generator shapes), `qlrs-cli` (budget flags) and `qlrs-bench`.
- New ADRs, SPEC requirements and TRACE rows.
- Multi-instrument synthetic traces, **only if** CP2.0 confirms the header and schema already support more than one instrument. Evidence that may point to this: `positions` is plural in `SyntheticCheckpoint`, and `InstanceKey` has a `scope`. If support is absent, record that and drop shape S5 to Part 3. Do not extend the header model in Part 2.

**Non-goals:** real-engine import or adapters, Parquet or columnar storage, UI, Mode E, multi-currency (`SyntheticCheckpoint.currency` stays singular), and LLM-based or transformation-based reduction (observed claims are never rewritten).

## 2. Assumptions

- The repository is at the end-of-Part-1 commit, with 42 tests passing, and is pinned to Rust 1.99.1 in `rust-toolchain.toml`.
- These crates have not been reviewed by a human and must be read before you change them: `qlrs-num`, `qlrs-schema`, `qlrs-bundle`, `qlrs-cli`, and `invariants.rs`.
- The performance numbers in the Part 1 handoff were never independently reproduced. Treat them as unverified.
- The development machine is a Mac (Apple Silicon). Linux CI runs gungraun and Kani.

## 3. First action (CP2.0, before any fix)

1. Read `AGENTS.md`, `docs/HANDOFF_PART_01.md`, `docs/SPEC.md`, `docs/TRACE.md`, ADR-0001 to ADR-0007, `docs/STACK.md`, and the protected catalogue and golden manifest.
2. Read the unreviewed crates listed in §2 and write `docs/review/PART1_UNREVIEWED_NOTES.md` covering any defects found. Report them; do not fix them yet.
3. Check whether Rust 1.100.0 is released (today is 5 Nov 2026; it is scheduled for 12 Nov). Do not bump the toolchain in Part 2 unless a human approves.
4. Reproduce every review finding as a committed test that asserts the **correct** behaviour and is marked `#[ignore = "R<n>: fixed at CP2.<m>"]`:

| ID | Finding | Reproduction |
|---|---|---|
| R1a | D04, 400 orders / 2,000 events | Reduction finishes in ≤ budget; record calls |
| R1b | D04, 2,000 orders / 10,000 events | Same |
| R1c | Long-lived order with partial fills across all windows, 12,000 events | Same |
| R1d | Carried-forward balance error, 10,000 events, `--target-seq` | Same |
| R2a | Fill after the last claim | Status INCONCLUSIVE, not CONSISTENT |
| R2b | Position with no claim after its latest fill | Status INCONCLUSIVE |
| R3 | D06 witness | Contains the anchor position claim and the fills between claims |
| R4 | Wrong position claim at every step, 100k events | Check is linear: wall time scales ~linearly from 10k to 100k |
| R5 | Mutation survivors listed in the review | Each has a killing test |

5. Add `scripts/known_failures.sh`. It runs `cargo test --workspace -- --ignored`, records which tests fail in `docs/review/known_failures_cp2.0.txt`, and commits the raw output.
6. Confirm or refute the suspected root cause of R1 by instrumentation. Count rejections by reason (new residual, new violation, lifecycle invalid) and commit the counts.

## 4. Mandatory agent rules

1. **Versions:** never write a version you have not verified on crates.io, docs.rs or an official release page the same day. Pin exact versions and record them in `docs/STACK.md` with the date checked.
2. **Benchmarks:** no performance claim may appear without committed raw output under `bench/results/CP2.x/`, including machine, OS, toolchain, commit and command.
3. **ADRs:** every change to a declared-stable interface requires an ADR merged in the same checkpoint.
4. **Tests and protected files:** never weaken tests, tolerances, the catalogue, golden files or ratchets to pass. If a fingerprint, golden file or expected status changes, **stop**, write `docs/review/APPROVAL_REQUEST_CP2.x.md` with a before/after diff and the reason, and wait.
5. **Blockers:** after two failed attempts at the same blocker, stop and report: what you tried, the evidence, and your options.
6. **Money and integrity:** no floats in money paths (the float-ban job stays on). Do not fabricate results, and do not create fake historical tags.
7. **Commits:** make one real commit per checkpoint, with the message `CP2.x: <objective>`. That commit must contain the code, tests, docs and raw outputs for the checkpoint. Tag it `part2-cp2.x` only after its pass criteria hold.
8. **Determinism:** output must be byte-identical across `RAYON_NUM_THREADS` ∈ {1, 2, 8} and across runs. Wall-clock budgets are the only allowed source of nondeterminism, and they must be reported as such.
9. **Unsafe and dependencies:** `forbid(unsafe_code)` stays in every crate. The dependency-direction script stays green.

## 5. Specification changes (extend the existing REQ scheme)

Use the next free numbers in the existing scheme. The `P2-` labels below are placeholders to map onto it, and each requirement needs a row in `docs/TRACE.md`.

| ID | Requirement |
|---|---|
| P2-CHK-01 | Evidence completeness: if any fill has no later cash claim on its account, **or** no later position claim on its instrument, the overall status is INCONCLUSIVE unless a VIOLATION exists. The report lists the uncovered seqs. |
| P2-CHK-02 | I2-local: the position residual is evaluated between consecutive position claims per (account, instrument), anchored at the observed earlier claim. Cumulative I2 is kept as a report-only finding. |
| P2-CHK-03 | Checker pattern extraction is O(n) overall. No per-violation rescan from event 0 is allowed. |
| P2-SCH-01 | `SyntheticCheckpoint` v1 adds `open_orders` (described in §7.1) and `schema_version`. |
| P2-BND-01 | `qlrs-bundle/v1` carries witness roles, checkpoint v1, the certificate and the compact log. The v0 reader is kept. |
| P2-RED-01 | Cut rule: the reduction window is (Anchor, Asserted], where Anchor is the latest observed claim of the same kind and scope before the target claim. |
| P2-RED-02 | Units are lifecycle suffixes after the cut, plus single claims. |
| P2-RED-03 | Dependency edges are those listed in §7.3. The FIFO edge is conditional on checkpoint inventory. |
| P2-RED-04 | Acceptance is exact `InstanceKey` equality plus anti-new-violation (§7.4). |
| P2-RED-05 | The default algorithm is SBR. ddmin-over-closures is available only as `--algorithm ddmin-closures`. |
| P2-RED-06 | Budgets: `--max-calls` and `--max-wall-ms`, with best-so-far semantics (§8). |
| P2-RED-07 | The certificate follows the semantics in §9 and can be re-checked by `qlrs bundle verify`. |
| P2-RED-08 | Results are deterministic across thread counts when the call budget is used. |
| P2-GEN-01 | The generator supports shapes S1–S8 (§11) with a stated distribution and a seed. |
| P2-QA-01 | Mutation testing covers `qlrs-reduce` and the fingerprint code, with a ratchet. |

## 6. Stable-interface changes (each needs an ADR)

| Interface | Change | ADR |
|---|---|---|
| `SyntheticCheckpoint` | Add `open_orders`, `schema_version`; `cut_seq` now means the anchor claim seq | ADR-0008 Checkpoint v1 |
| `qlrs-bundle/v0` | Becomes `v1`; v0 stays readable, and v0 bundles verify under the v0 rules | ADR-0009 Bundle v1 |
| `UnitGraph` | Suffix units, conditional FIFO edges, claim → contributor edges, CSR built from sorted edges | ADR-0010 Unit model v2 |
| `reduce` | Returns `ReduceOutcome {witness, certificate, stats, budget_exhausted, closure_1_minimal}` | ADR-0011 SBR and budgets |
| `Checker::check` / `Finding` | I2-local finding kind, evidence-completeness report and INCONCLUSIVE reasons | ADR-0012 Evidence completeness and I2-local |
| Fingerprint bytes | **Only if** I2-local changes class fingerprints: version the prefix to `qlrs-fp2:` and keep `qlrs-fp1:` computable | ADR-0013, plus human approval |
| `Predicate::test` | Unchanged signature; add `FastPredicate` as a separate trait | No ADR needed (additive) |

## 7. Domain specification

### 7.1 Checkpoint v1 (fields)

- `schema_version: u16` (= 1)
- `provenance` (fixed text "synthetic: computed by qlrs-ledger from original prefix")
- `account_id`, `currency`
- `cash: Fixed`
- `positions: [(instrument_id, Fixed)]`
- `open_orders: [OpenOrderState]`, where `OpenOrderState` has these fields:
  - `client_order_id`
  - `instrument_id`
  - `side`
  - `status`, one of the lifecycle table states
  - `order_qty: Fixed`
  - `filled_qty: Fixed`
  - `last_event_original_seq`
  - `trade_ids_seen_digest`, a BLAKE3 hash of the sorted trade ids, needed so that duplicate-trade-id detection works across the cut
- `cut_seq` (the anchor claim's original seq), `source_digest`, `description`

### 7.2 Witness roles

| Role | Content | Asserted? | Source |
|---|---|---|---|
| Anchor | Observed claim opening the window | No (opening balance) | Original bytes, verbatim |
| Asserted | Target observed claim | Yes | Original bytes, verbatim |
| Evidence | Kept window events | — | Original bytes, verbatim |
| Synthetic | Checkpoint v1 | — | Computed by the ledger and labelled |

### 7.3 Dependency edges

| Edge | Condition |
|---|---|
| Lifecycle successor → predecessor | Same `client_order_id`, both after the cut |
| Claim → contributor | The contributor is inside the claim's window and affects that claim's account or instrument |
| Sell suffix → earlier buy suffix (FIFO) | Only if checkpoint inventory + kept earlier buys < sell qty |
| Suffix → checkpoint open-order entry | Implicit; must exist if the suffix starts with a non-initial event |

Build the CSR from a `Vec<(u32,u32)>` that is sorted and deduplicated. Reverse CSR is needed for removal closures.

### 7.4 Acceptance (the slippage guard)

- The target `InstanceKey` must be present exactly.
- Every other finding's `InstanceKey` must be in `original_keys`, the set of keys from the full-trace report.
- The final witness is always re-verified by the full `Checker`. A fast-evaluator acceptance on its own is never final.

### 7.5 Statuses

| Status | Rule |
|---|---|
| VIOLATION | Any finding |
| INCONCLUSIVE | No violation, but P2-CHK-01 lists uncovered fills, or another existing reason applies |
| UNSUPPORTED | As in Part 1 |
| CONSISTENT | No violation and full evidence coverage |

Exit codes are unchanged. If any catalogue defect's expected status changes as a result of P2-CHK-01, stop and request approval (rule 4).

## 8. Budgets and best-so-far

- **Flags:** `--max-calls <u64>`, default 100,000, counting full-Checker calls (fast-evaluator probes are counted separately), and `--max-wall-ms <u64>`, with no default.
- **Invariant:** the current kept set is always a validated witness. The search starts from the full window and commits only validated candidates.
- **On exhaustion:** return the current witness with `budget_exhausted = true`, `closure_1_minimal = false` and a partial certificate covering the closures already tested. The status stays VIOLATION. The CLI prints which budget fired.
- **Determinism:** call-budget exhaustion is deterministic; wall-clock exhaustion is not and must be labelled.

## 9. Certificate semantics

`Certificate {predicate_id, instance_key, unit_graph_digest, kept_units, entries: [CertEntry]}`, where `CertEntry` has these fields:
- `removed_closure`: the RLE unit ranges
- `outcome`, one of:
  - `ResidualChanged {new_residual}`
  - `NewViolation {key}`
  - `LifecycleInvalid {sub_code}`
  - `KeyMissing`
- `evaluator`: `fast` or `full`

**Claim:** for every kept unit u, there is an entry whose `removed_closure` equals u's reverse closure within the kept set, and that entry's outcome is not acceptance. `closure_1_minimal = true` only if this holds for every kept unit and no budget fired. `qlrs bundle verify --recheck-certificate` re-runs every entry with the full Checker and must reproduce each outcome exactly.

## 10. Algorithm specification (SBR)

| Step | Action | Calls |
|---|---|---|
| 1 | Run the full check, select the target `InstanceKey` (`--target-seq` or the first violation), and record `original_keys` | 1 |
| 2 | Find the Anchor and set the cut; compute checkpoint v1 by ledger replay of the full prefix | 0 |
| 3 | Build suffix units, edges and CSR; compute closures | 0 |
| 4 | Build the contribution table for the target window. Seed R = units with non-zero contribution, or units in the violating lifecycle for I3; S0 = closure(R) | 0 |
| 5 | Verify S0 with the full Checker. If it passes, go to step 7 | 1 |
| 6 | Binary Reduction: S = S0, D = remaining closures sorted by cost C(S ∪ {d}) = event count of the union, ties broken by lowest unit id. Repeat: find the minimal r with P(S ∪ D[..=r]) using prefix sums (fast) and confirm with the full Checker; S ∪= D[r]; D = D[..r] re-sorted; until P(S) holds | O(s log n) |
| 7 | Certificate pass: for each kept unit, try removing its reverse closure. In parallel, the lowest index wins; re-run if a removal is accepted | k |
| 8 | Final full-Checker re-verification; emit witness, certificate and log | 1 |

**Speculative parallelism.**
- In step 6, on the full-Checker path, split the search interval into p+1 parts and evaluate p probes on rayon.
- The committed r is the lowest succeeding index. Probes above an already-succeeded index are cancelled via `AtomicBool`.
- Results do not depend on timing, because the outcome is a function of the predicate values alone.

**Fallback.** If the monotonicity probe or a differential test finds that P is non-monotone on a shape, record it. SBR remains sound, because every commit is validated; it is only possibly less small. Use `--algorithm ddmin-closures` as a comparison on that shape.

## 11. Generator specification (qlrs-testkit)

| Shape | Description | Default distribution |
|---|---|---|
| S1 sequential | Part 1 shape | 1 instrument, strictly sequential orders, claims after every fill |
| S2 interleaved | k open orders at once | k ~ uniform{2..16}; events interleaved round-robin with seeded jitter |
| S3 long-lived | One order partially fills across all windows | 1 parent order; fills of size 1 lot; claims every m fills |
| S4 sparse claims | Claims after every m-th fill | m ~ uniform{5..50}; final claim present unless the coverage test is disabled |
| S5 multi-instrument | i instruments, single currency | i ~ uniform{2..8} (only if the header supports it) |
| S6 carried-forward | Engine carries an earlier cash error forward; a second defect later | Two planted defects, `--target-seq` on the second |
| S7 dense violations | Wrong position claim at every step | 100% of position claims perturbed |
| S8 FIFO chains | Long buy ladders followed by one large sell | Ladder length ~ uniform{10..500} |

Every shape takes `(seed, n_events, defect)`. The distributions are written into `docs/GENERATOR.md`. The protected catalogue D01–D14 is unchanged, and new shapes reuse the existing defect injectors.

## 12. Performance targets and baselines

- Record baselines at CP2.0 (Part 1 reducer) and at CP2.12 (SBR) with the same commands, on two machines:
  - Mac M1 or later, using criterion for wall time and samply for profiles
  - Linux CI, using criterion plus gungraun 0.20.0 (or the version re-verified at implementation time) for instruction counts
- **Proposed goals (estimates, not measurements):**
  - the evaluator answers a prefix query in < 100 ns
  - SBR on any S1–S8 trace at 10k events finishes in < 1 s, with ≤ 200 full-Checker calls
  - at 100k events, it finishes in < 5 s on 8 threads
  - the R4 check at 100k finishes in < 0.5 s, with report size linear in the number of violations
- Raw outputs go in `bench/results/`. A summary table in `docs/BENCH_PART_02.md` lists defect × shape × {kept events, exhaustive minimum where computable, full calls, fast probes, wall ms}.

## 13. Log format

`qlrs-reduce-log/v1` is JSONL, one line per committed step and one per certificate entry:
- `{step, phase, removed: [[lo,hi],…], kept_units, kept_events, calls_full, probes_fast, outcome}`

Removed-unit ids are run-length encoded ranges and are never listed one by one. The size of the log must be recorded for the R4 trace.

## 14. Checkpoints

Each checkpoint lists its tasks, the crates it touches, how to verify it, and its pass/fail criteria. Every checkpoint ends with one commit, `CP2.x: …`, and it **stops** if any criterion fails twice.

**Stage A: correctness**

- **CP2.0 — Reproduce.** Covers §3. Crates: tests only. Verify with `cargo test --workspace`, then `scripts/known_failures.sh`. **Pass:** all 42 existing tests pass; every R-test exists, is ignored, and fails when run with `--ignored`; the root-cause instrumentation counts are committed; and the unreviewed-crate notes exist.
- **CP2.1 — Evidence completeness.** Implement P2-CHK-01 in `qlrs-check`. **Pass:** R2a and R2b are un-ignored and pass; the catalogue's expected statuses are unchanged, or else an approval request has been filed and the work is stopped.
- **CP2.2 — Linear checker and compact logs.** Implement P2-CHK-03 and the §13 log format in `qlrs-check` and `qlrs-reduce`. **Pass:** R4 is un-ignored; the 10k→100k wall-time ratio is ≤ 12 (raw output committed); report and log sizes are recorded.
- **CP2.3 — Checkpoint v1 and bundle v1.** P2-SCH-01 and P2-BND-01, with ADR-0008 and ADR-0009. Crates: `qlrs-schema`, `qlrs-bundle`, `qlrs-cli`. **Pass:** round-trip property tests pass; Part 1 v0 golden bundles still pass `qlrs bundle verify`.
- **CP2.4 — I2-local and witness roles.** P2-CHK-02 and §7.2, with ADR-0012. **Pass:** R3 is un-ignored; the D06 witness contains Anchor, Evidence fills and Asserted. If fingerprints change, follow rule 4.
- **CP2.5 — Cut, suffix units, edges and CSR.** P2-RED-01 to 03, with ADR-0010. **Pass:** a property test shows every candidate built from a closed set is lifecycle-valid against the checkpoint; R1a–R1d finish within the default call budget using the *existing* search.
- **CP2.6 — Budgets.** P2-RED-06. **Pass:** a test where the budget fires returns a valid witness with `closure_1_minimal = false`; results are byte-identical across thread counts under `--max-calls`.
- **CP2.7 — Generator shapes.** §11, in `qlrs-testkit`. **Pass:** every shape × D01–D11 combination generates, and its expected status matches the catalogue; `docs/GENERATOR.md` is written. **Gate A:** all R-tests pass.

**Stage B: the flagship algorithm**

- **CP2.8 — Algebraic evaluator.** Contribution tables and prefix sums, plus differential tests against the full Checker (≥ 10,000 proptest cases per shape in CI and a nightly long run). Kani harness for the prefix-sum lemma (optional Linux job). **Pass:** zero differential mismatches.
- **CP2.9 — SBR and certificate.** §9 and §10, with ADR-0011. **Pass:** idempotence, order-independence and validity properties hold; `--recheck-certificate` reproduces every entry; kept/min ≤ 1.5 on traces with ≤ 20 units (proposed goal).
- **CP2.10 — Speculative parallelism and cache.** The 128-bit incremental hash and collision-checked cache, the p-ary search, and cancellation. **Pass:** determinism across 1, 2 and 8 threads; a cache test with a forced-collision hash function proves no false hit.
- **CP2.11 — Mutation ratchet.** Run cargo-mutants 27.1.0 (re-verify the version) on `qlrs-reduce`, `qlrs-check` and the fingerprint code; set `.cargo/mutants.toml` with `timeout_multiplier`; add a PR job using `--in-diff`. **Pass:** every survivor named in the review is killed; the new caught/viable ratio is committed as a ratchet; remaining survivors are listed in `docs/MUTANTS_PART_02.md` with a justification each.
- **CP2.12 — Gate B-synthetic and baselines.** Covers §12 and §15. **Pass:** every gate cell passes, or misses are reported honestly with a stop.
- **CP2.13 — Handoff.** Write `docs/HANDOFF_PART_02.md` using §16. **Pass:** the Definition of Done in §17 holds.

## 15. Gate B-synthetic

The matrix is D01–D11 × S1–S8 (S5 only if supported), at sizes 1k, 10k and 100k events, with seeds {1, 2, 3}. Mandatory regression cells: R1a, R1b, R1c and R1d.

| Criterion (proposed goal) | Threshold |
|---|---|
| Validity and exact key | 100% of cells |
| Kept events | ≤ 32, or ≤ 2× the exhaustive minimum where computable |
| Wall time (release build, 8 threads, Linux CI) | ≤ 1 s at 10k events; ≤ 5 s at 100k events |
| Full-Checker calls | ≤ 200 at 10k events |
| `closure_1_minimal` | true in 100% of cells, with no budget fired |
| Determinism | Identical bundles across 1, 2 and 8 threads |

Any threshold that is missed must be reported with its raw numbers, and the threshold must not be changed.

## 16. Handoff template (`docs/HANDOFF_PART_02.md`)

The handoff document has these sections:
1. Summary.
2. Checkpoint commits table: CP, commit hash, tag, and pass evidence path.
3. R-findings: root cause confirmed or refuted, and the fix.
4. Interface changes and ADRs.
5. Approval requests and their outcomes.
6. Gate B table, with links to raw output.
7. Baselines before and after.
8. Mutation results and survivors.
9. Known limitations, including non-monotone shapes, a wall-clock nondeterminism note, and an S5 status.
10. Verified versions with dates.
11. Open questions for Part 3.

## 17. Definition of Done

- All R-tests pass, and the test-count ratchet has increased.
- Gate B-synthetic passes, or is honestly stopped.
- Differential mismatches are zero.
- The mutation ratchet has been updated.
- ADR-0008 to ADR-0012 (and ADR-0013 if needed) are merged.
- SPEC and TRACE are updated.
- Every checkpoint has exactly one commit.
- No unsafe code, no floats in money paths, and the determinism job is green.
- The handoff is written.

## 18. Interfaces Part 3 can rely on

- Generator shapes S1–S8 with `(seed, n_events, defect)`.
- `ReduceOutcome`, the certificate and `qlrs-reduce-log/v1`.
- Checkpoint v1 and `qlrs-bundle/v1`, with the v0 reader.
- `FastPredicate` and the differential test harness.
- `--max-calls` and `--max-wall-ms` semantics.
- The bench result layout and the baseline numbers, so that Part 3's 10⁶–10⁷-event scale work starts from measured data.

---

**Caveats.**
- The monotonicity of P under the new edges is a design argument and has not been proven. CP2.8 and CP2.9 test it empirically.
- All of the following are proposed goals and estimates, not measurements: the expected call counts, the < 1 s and < 5 s targets, and kept/min ≤ 1.5.
- Some versions were verified only from secondary pages, such as release trackers and dependency-update PRs, so the agent must re-check them.
- Bolero, cargo-fuzz, roaring and bumpalo were not verified.
- The competitor scan for this part was not done.