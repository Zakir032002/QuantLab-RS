# Part 2 handoff

## 1. Delivered work and acceptance status

The rebased Part 2 implementation is on `main`. Production source and all 82
native tests are committed at **766632a0d3dc8bc2a160cf9eba81a945e129db63**;
**4941acd858a82ca112b4b134f6a86b1bae5b0729** adds benchmark and gate records
without changing Rust sources. The 891-cell synthetic gate, differential checks,
offline certificates and all three mutation floors pass.

**Overall Part 2 acceptance remains incomplete:** the separate R4 dense-error
10k-to-100k scaling ratio is 12.2346, above its unchanged limit of 12. CP2.2
stopped after two attempts. Consequently this handoff does not claim a passed
CP2.13 or create its passed tag. [BLOCKERS.md](BLOCKERS.md) records the stop.
The Part 2 checkpoint rule (§14) says a checkpoint “stops if any criterion fails
twice.” Both attempts are recorded in `docs/BLOCKERS.md`; independent work proceeded.

The authoritative contract is [SPEC.md](SPEC.md), with traceability in
[TRACE.md](TRACE.md). [PART_02_REBASE.md](PART_02_REBASE.md) overrides stale
names, counts, ADR numbers and gates in the uploaded instructions.

## 2. Checkpoint commits

Each implemented checkpoint has one commit; prior Stage A work was reused.

| Checkpoint | Commit | Passed tag | Evidence |
|---|---|---|---|
| CP2.0 precedence and scoped evidence | cf0bb5ffd92f259712f5a1cf42b1992e7e1ed774 | part2-cp2.0 | [CP2.0](../bench/results/CP2.0/), [review](review/known_failures_cp2.0.txt) |
| CP2.1–CP2.7 | Reused prior work and regressions in CP2.0/CP2.8 | No artificial tags | [R-findings](review/R_FINDINGS_PART2.md); CP2.2 specifically stopped |
| CP2.8 algebra and closure search | 3d43d59ffc0595868079809aebd1c156bb49fd50 | part2-cp2.8 | [commands and tests](../bench/results/CP2.8/COMMANDS.md) |
| CP2.9 compact logs and offline proofs | 916747a74b7ff96e379d7a495ab4f8d8d10a17c0 | part2-cp2.9 | [commands and proofs](../bench/results/CP2.9/COMMANDS.md) |
| CP2.10 cache and speculation | a877c7bcf0435566073b4bc22a0a4094feaedfe6 | part2-cp2.10 | [commands and determinism](../bench/results/CP2.10/COMMANDS.md) |
| CP2.11 scoped mutation ratchets | 766632a0d3dc8bc2a160cf9eba81a945e129db63 | part2-cp2.11 | [complete campaign and rechecks](../bench/results/CP2.11/COMMANDS.md) |
| CP2.12 synthetic gate and measurements | 4941acd858a82ca112b4b134f6a86b1bae5b0729 | part2-cp2.12 | [raw provenance](../bench/results/CP2.12/COMMANDS.md), [891 cases](../bench/results/CP2.12/GATE.md) |
| CP2.13 handoff | The commit containing this file | None: acceptance incomplete on R4 | This document and BLOCKERS.md |

Historical `cp1.8` is unchanged. No checkpoint history was manufactured for the
already completed Stage A work.

## 3. R1–R5 findings

[Individual findings](review/R_FINDINGS_PART2.md) distinguish confirmed problems
from assumptions refuted on the starting checkout. Fresh mandatory rows are in
[regressions.jsonl](../bench/results/CP2.12/regressions.jsonl).

- R1a/b already met the bound on starting source; retain regression coverage.
  Their fresh 2000/10000-event runs each keep five events with two candidate calls.
- R1c restores status and filled quantity for long-lived orders at the claim
  checkpoint: a 12053-event input keeps a one-event suffix.
- R1d preserves the observed opening error and the exact later target; earlier
  faulty claims are opening balances, not new witness accusations.
- R2 now lets a proven violation outrank uncovered-tail findings and retains
  those diagnostics. Without a proven violation, missing evidence is inconclusive.
- R3 checks consecutive claims of the same instrument. I2 witnesses omit other
  assets and cash claims unless lifecycle/model support requires them. The saved
  funding fixture proves an unrelated asset's sale can be necessary cash support.
- R4 removes dynamic iterator dispatch, prepares order limits once and reserves
  dense finding output. Pattern counts were already incremental. Absolute 100k
  time is 26.998455 ms; scaling still misses 12. Both attempts are retained.
- R5 covers batch choice, dependent closure removal, caps, pins, no-ops, forced
  hash collisions and fingerprint encoding. All named viable helper mutants die.

The public cache-bounds suspicion was refuted: intersection already normalizes
bounds. Its incorrect assertion and correction are retained. The final regression
tests that public contract plus a defensive private-batch precondition; no public
cache bug is claimed. See [ADR-0015](adr/0015-mutation-builds-and-fixed-cache-domain.md).

## 4. Interfaces and decisions

The CLI preserves `--time-limit-ms`, `--max-calls` and checkpoint `orders`/`trades`.
Default reduction is SBR; `--algorithm ddmin-closures` retains the comparison
algorithm. Defaults remain 30 seconds and 10000 candidate calls. Exhaustion
returns the best verified witness, `closure_1_minimal: false` and a stop reason.

Full-trace verification checks evidence completeness. Witness checking restricts
financial evidence to the exact target's scope, with full lifecycle/model
admissibility. Unsupported precedence and exact instance matching remain intact.

Checkpoint schema version 1 retains observed opening claims independently of
expected ledger state. Bundles write v1 with original-byte roles, RLE logs and a
versioned closure certificate; v0 reading is tested against a bundle produced by
the actual pre-Part-2 writer. [Compatibility provenance](../fixtures/compat/README.md).
`qlrs bundle verify PATH --recheck-certificate` independently rebuilds the graph
and rechecks each entry. Zero tested removable candidates is explicitly vacuous.

Minimal pins are the necessary observation boundaries and target. CSR indexes
use original unit identities with compact window-local rows. Checked algebraic
seeds and prefix unions propose removals; full checking admits every committed
candidate. A 128-bit incremental cache hash always checks the exact retained
unit set on a hit. This cache hash does not change the class fingerprint codec.

Speculative batches choose the lowest accepted candidate independent of workers;
higher results, errors and cache effects are discarded. Reported full slots
conservatively reserve whole batches and can exceed actual executed checks.
The candidate cap includes cache hits; mandatory initial/context/final validation
is outside it. Wall limits are cooperative and cannot interrupt a running replay.

New ADRs start at 0010, preserving occupied 0008/0009:
[0010 precedence](adr/0010-verdict-precedence-and-witness-scope.md),
[0011 formats and indexes](adr/0011-checkpoint-bundle-and-compact-window-indexes.md),
[0012 search](adr/0012-algebraic-seeds-search-and-certificate.md),
[0013 logs and proof verification](adr/0013-compact-logs-and-offline-certificate-validation.md),
[0014 cache and speculation](adr/0014-incremental-cache-and-deterministic-speculation.md),
[0015 build caching and internal domain](adr/0015-mutation-builds-and-fixed-cache-domain.md).

## 5. Approval outcomes

The user's rebase explicitly authorized: “A proven VIOLATION outranks missing
later evidence. Keep the missing-evidence findings” and “Update SPEC precedence
and req_check_011 … it is a correction, not a weakened test.” ADR-0010 records
that correction. No golden file, defect catalogue, rounding policy, tolerance or
class-fingerprint encoding changed. No new permission stop was introduced.

The existing ADR-0009 authorizes the labeled Intel Mac fallback after ARM runner
capacity failures; current M1 measurements are not claimed.

## 6. Gate B and verification

The [891-row table](../bench/results/CP2.12/GATE.md) and
[machine-readable output](../bench/results/CP2.12/gate.jsonl) cover D01–D11 ×
S1–S9 × 1k/10k/100k budgets × seeds 1/2/3. All pass exact-key validity,
per-case event bounds, complete recheckable certificates, time/call limits and
byte-identical bundles with 1/2/8 workers. Actual event counts are recorded.

All 891 small windows equal their computed exhaustive event minimum. 564 cases
test actual removable certificate candidates; others are explicitly vacuous.
Maximum full candidate slots: 10. Slowest 100k-budget case: 148.109126 ms.
The extra sparse regression legitimately retains 104 of 10211 events and tests
26 certificate candidates; the removed universal 32-event bound is not applied.
S9 claims are staggered and require no shared cash/position frontier. S5's
multiple-instrument shape is supported and participates in the full matrix.

The native suite is **82 passed, zero skipped**, up from 60. Traceability covers
81 requirements. The latest 10000-case-per-invariant-per-shape differential run
checks 180000 closed subsets with zero mismatches; nightly uses 100000 cases.
Normal clippy, doctests, catalogue, protected goldens and 400000 independent
numeric comparisons pass. [Native evidence](../bench/results/CP2.11/final/).

All five hosted final-source jobs passed: Linux and Intel Mac quality, Miri,
scale/instructions and Mac benchmarks. The direct observation is recorded in
[hosted-ci-final.json](../validation-results/part2/hosted-ci-final.json), for
[run 41936116348](https://github.com/Zakir032002/QuantLab-RS/actions/runs/41936116348).
Earlier a877c7b success is retained separately and is not substituted for this
final-source pass.
[Source hashes](../validation-results/part2/final-source-sha256.txt) and the
[equivalence record](../validation-results/part2/source-equivalence.json) show
the handoff changes only documentation, measurement records and CI observations.

## 7. Before and after measurements

[BENCH_PART_02.md](BENCH_PART_02.md) links raw 50-sample Criterion estimates,
machine/compiler records, exact source snapshots and the recorded patch. Before
Linux source is reconstructed exactly from 281fa154 and measured on the same
host with identical compiler/flags; it is not backdated as a CP2.0 execution.
Measured runtime and benchmark sources match final source, excluding the
subsequently corrected trailing test module, as the recorded equivalence shows.

| Shared Linux workload | Before median | After median |
|---|---:|---:|
| Checker, 100k events | 2.995 ms | 2.202 ms |
| Reduction, 100k events | 6.289 ms | 4.651 ms |
| Checked prefix query | No Part 1 equivalent | 7.18 ns |

The committed Intel Mac records identify exact commits and verified
transfer/file/input hashes. Final-source 4941acd medians are 4.376 ms checker and
11.906 ms reducer, versus 8.966/19.656 ms before. Final replay is 0.258 ms versus
0.317 ms before; the earlier development run's 0.347 ms regression is also
retained explicitly. [Final-source raw data](../bench/results/CP2.12/macos/final-source/).
Same declared hardware does not establish the same physical host. No universal
throughput promise is inferred.

Linux instruction counts are 1297 parse / 125 multiply-rescale / 33519 replay;
original instruction ratchets pass. R4's independent ratio gate remains failed.
Current M1, samply profiles and a Kani proof are not measured locally.

## 8. Mutation results

[MUTANTS_PART_02.md](MUTANTS_PART_02.md) individually lists all survivors and
their coverage gaps or specific equivalence arguments. None is silently dropped.

| Separate scope | Caught / viable | Starting floor | Result |
|---|---:|---:|---|
| Numeric/checker | 359/399 | 347/387 | Pass |
| Reducer | 190/270 | 78/117 | Pass |
| Fingerprints (also included in checker) | 11/11 | 11/11 | Pass |

The complete 743-mutation run used 81 tests. The new 82nd test kills three
checker survivors on exact-site rechecks. Three original timeouts were rerun
serially at the same 60-second cap: two survive, one is caught. The explicit
derived complete-coverage report retains all 743 entries and raw-file hashes:
549 caught, 120 missed, 74 unviable, zero unresolved timeouts. It is not presented
as a second raw full campaign. Raw results and compressed individual logs/diffs
are retained; incomplete attempts contribute nothing. Every viable
evaluate_batch/remove_closure mutant is caught (five caught, two unviable).

Nightly runs all scopes with workspace tests and separate nondecreasing floors;
PR `--in-diff` runs flag new unreviewed survivors without pretending to be a full
campaign. Exact-site reviewed survivors remain counted nightly. Test optimization
is disabled only for mutation build cost; overflow/debug assertions stay enabled.

## 9. Known limitations

- R4 scaling is stopped and fails; CP2.13's all-R acceptance remains incomplete.
- Lifecycle validity, cash funding and anti-new-violation admission are
  non-monotone. Saved non-monotone/funding fixtures exercise actual fallback;
  algebra never replaces final full checking. Closure-1-minimality is relative
  to the stated pins/graph, not global minimality on arbitrary large inputs.
- Completed and call-limited searches are deterministic. A wall deadline can
  stop at different candidates under different scheduling or machine load.
- CSR and unit-event preparation use partition_point and the target window;
  initial input validation/checkpoint replay still traverses history, and the
  global unit bitmask is O(total units / 64). Total setup is not O(window).
- Original JSONL remains resident. Earlier whole-process memory and parse-rate
  targets remain limitations; streaming/mmap storage is future work.
- Checkpoints are conditional synthetic state; hashes establish integrity,
  not authorship. Mode T does not re-run an engine. The declared single-currency
  spot model still excludes margin, shorting and unsupported order types.
- Kani is optional and unexecuted locally. Its bounded prefix-union harness
  does not prove the entire reducer or predicate monotonicity.

## 10. Verified versions

Rust 1.99.1 remains pinned; see [STACK.md](STACK.md) for dependency versions,
release dates and official registry checks. On 2026-11-04 UTC, Part 2 reverified
Rayon 1.12.0 (released 2026-04-14), cargo-mutants 27.1.0 (2026-06-02) and optional
kani-verifier 0.68.0 (2026-09-16). Raw registry metadata is in CP2.8/CP2.11.
Miri uses nightly-2026-11-04; benchmark configuration retains Criterion 0.8.2
and gungraun 0.20.0. docs.rs remains blocked by runtime policy; ADR-0004's
official-registry fallback is recorded, without bypassing TLS or destination rules.

## 11. Part 3 starting points

Preserve public shape generation, exact target keys, window contribution tables,
checkpoint orders/trades, v0 compatibility, v1 roles/certificate versioning and
the unchanged budget flags. Expand adversarial large-window/non-monotone search,
certificate tampering coverage and the explicitly listed mutation gaps. Address
R4 with a separately reviewed continuation, rather than relabeling this miss.
Measure memory/streaming and acquire actual M1/profile/proof evidence before
making those claims. Cache pinned CI tools to shorten validation setup. Engine
adapters, multi-currency and Mode E remain separate scope decisions.
