# Claim-boundary corrections handoff

The ten requested corrections are implemented on `main`. Final production source is
`e348f56`; source commitments and measured results are retained in the
[Linux record](../bench-results/2026-11-04-lookup-linux-x86_64/).
The original `cp1.8` tag and its benchmark records remain a historical baseline.
The normative contract is [SPEC.md](SPEC.md); [ADR-0008](adr/0008-claim-boundaries-and-bounded-reduction.md)
records the user-authorized semantic changes.

| Requested correction | Delivered behavior |
|---|---|
| Claim checkpoint and open orders | Cut at the target's preceding relevant claim. Carry independent expected cash/positions, order metadata, status, filled quantity and relevant prior trade IDs. A long-lived order need not keep its submitted/accepted prefix. |
| Opening claim | Preserve the earlier observed claim as the opening balance without re-auditing it. Expected ledger state remains independent; separately carry pending movement since each scope's last claim. |
| Inventory dependencies | Consume checkpoint inventory first; keep post-cut buy prerequisites only for uncovered sell quantity. |
| Bounded search | `--time-limit-ms` and `--max-calls` return the best verified witness, `closure_1_minimal: false` and a reason on exhaustion. Defaults: 30 seconds / 10,000 calls. |
| Missing later evidence | A fill without a later cash claim or later same-instrument position claim makes the overall result `INCONCLUSIVE`; earlier fully evidenced findings remain diagnostics. |
| Local I2 | Compare consecutive same-instrument position claims using signed intervening fills. Pin those fills and the opening boundary in the witness. Carried errors are not repeatedly accused. |
| Counts and logs | Incremental per-window/per-order kind counts; sparse complete checking for I1/I2/I3; constant-size removed-unit counts in logs. Legacy removed-ID arrays remain readable. |
| Rich generator | Three profiles with interleaved partial fills, long-lived orders, sparse claims, 3/4 instruments and carried-forward errors. Seed affects prices and scheduling. |
| Expanded gate | D01–D11 on each profile, more than a million events per trace, at most 128 kept events, 2000 ms reduction time and 1024 candidate calls. Exact-key replay, original bytes and a completed minimality certificate are mandatory. |
| Mutation coverage | Full workspace tests against numeric, checker, fingerprint and reducer mutations. Independent new-scope floors preserve the legacy numeric/checker floor. |

## Verification

All **60 native tests pass with zero skips**; the test-count floor is 60.
The original protected defect catalogue, golden files, rounding policy and
instruction baseline are unchanged. New coverage includes local I2 and carried
errors, checkpoint order/trade tampering, missing evidence, malformed contexts,
zero/call/time limits, bundle replay, legacy logs and randomized sparse/dense
checker comparisons against an independent pattern counter.

Reports and every bundle byte match with one/eight Rayon threads for classic
traces, all eleven rich defects and a call-limited result. A time cutoff can
stop at different candidates under different scheduling; completed and
call-limited searches retain deterministic ordering.

The final routine suite and catalogue are in
[latest validation](../validation-results/order-lookup/). High-volume numeric
properties remain native. All five hosted retry jobs passed: Linux quality,
Intel macOS quality, Miri, scale/instructions and Intel Mac benchmarks.
[Successful workflow](https://github.com/Zakir032002/QuantLab-RS/actions/runs/41898170782),
[directly observed statuses](../validation-results/order-lookup/hosted-ci.json).
The checkout is `5e1faad`, whose Rust/manifest hashes match production source
`e348f56`. Earlier ARM Mac jobs were cancelled while queued because GitHub could
not acquire runners; they are not counted as passes.
[Provider diagnostics](../validation-results/order-lookup/mac-cancellation.json)
and [ADR-0009](adr/0009-macos-runner-capacity-fallback.md) record the Intel fallback.

The broader `111b1db` baseline previously completed Linux/Mac quality, Miri,
scale/instruction and actual M1 measurements:
[completed baseline workflow](https://github.com/Zakir032002/QuantLab-RS/actions/runs/41885851897),
[observed statuses](../validation-results/claim-boundaries/hosted-ci.json).

## Measured Linux results

Shared Linux x86_64 / AMD EPYC 9V74; Rust 1.99.1; portable target, fat LTO,
checked arithmetic, system allocator. Parse uses five Rayon workers; replay is
single-threaded. Criterion has 50 samples per workload. The stored median and
its confidence interval are used below.

| Workload | Median | Median confidence interval |
|---|---:|---:|
| Decimal parse | 68.113 ns | 67.694–69.110 ns |
| i128 multiply/rescale | 11.983 ns | 11.914–12.113 ns |
| Wide multiply/rescale | 469.525 ns | 453.201–489.050 ns |
| JSONL parse, 100k events | 68.316 ms | 66.622–72.486 ms |
| Independent replay, 100k events | 179.124 µs | 174.234–184.859 µs |
| Full check, 100k events | 2.766 ms | 2.726–2.818 ms |
| D03 reduction, 100k events | 5.454 ms | 5.377–5.589 ms |

The richer checker does more work than the historical implementation and its
classic full-check measurement is slower. The reducer avoids repeated pattern
rescans on long windows. These shared-host observations do not establish speed
on every trace or machine.

Raw [samples/estimates](../bench-results/2026-11-04-lookup-linux-x86_64/criterion/),
[console](../bench-results/2026-11-04-lookup-linux-x86_64/criterion.txt),
[commands and timing scope](../bench-results/2026-11-04-lookup-linux-x86_64/COMMANDS.md),
[machine](../bench-results/2026-11-04-lookup-linux-x86_64/machine.json),
[compiler](../bench-results/2026-11-04-lookup-linux-x86_64/rustc.txt),
[source hashes](../bench-results/2026-11-04-lookup-linux-x86_64/source-sha256.txt).

Final Valgrind Ir counts are **1297** (decimal parse), **125** (multiply/rescale)
and **34519** (1000-event replay). The unchanged 5% ratchet passes. An initial
range implementation measured 38525 replay instructions; a separate full-range
loop restored compiler bounds-check elimination.
[Both raw runs](../bench-results/2026-11-04-lookup-linux-x86_64/).

All **33 extended gate cases pass**, plus three large clean controls. Inputs
have at least **1,078,146 events**; the largest witness keeps **70 events**,
the slowest reduction takes **203.554866 ms**, and each case records two
candidate calls / 324 log bytes. These are observed maxima for this run, not
universal guarantees. [Raw extended gate](../bench-results/2026-11-04-lookup-linux-x86_64/rich-gate.txt).
The original million-event D01/D03/D07 gate still keeps six observed events in
each case. [Raw original gate](../bench-results/2026-11-04-lookup-linux-x86_64/scale-gate.txt).

Earlier source `111b1db`, **Apple M1 (Virtual)**, three CPUs / 7 GiB RAM,
macOS 15.8.2, Rust 1.99.1:

| macOS workload | Median | Median confidence interval |
|---|---:|---:|
| JSONL parse, 100k events | 67.011 ms | 54.014–79.229 ms |
| Independent replay, 100k events | 197.929 µs | 185.910–203.655 µs |
| Full check, 100k events | 2.318 ms | 2.260–2.399 ms |
| D03 reduction, 100k events | 6.711 ms | 6.007–7.196 ms |

All seven workloads have 50 raw samples. This M1 record's source and input
hashes match the separate `111b1db` Linux record exactly; its file manifest and public transfer commitment
are verified. Parsing uses five Rayon workers on three virtual CPUs and remains
below the original 300 MB/s engineering goal on this runner. The wide interval
also shows shared-runner variability. Final-source M1 results are not measured
during the ARM capacity outage. The latest shared-Linux parse observation is
also below the 300 MB/s reference goal.
[Raw macOS samples, medians and machine/compiler records](../bench-results/2026-11-04-claim-boundaries-macos-arm64/),
[verified transfer](../bench-results/2026-11-04-claim-boundaries-macos-arm64/TRANSFER.json).

The final-source retry completed benchmarks on **Intel Core i7-8700B**, four
CPUs / 14 GiB RAM, macOS 15.7.9, Rust 1.99.1. This is the standard Intel runner
selected after the ARM capacity cancellation; these are not M1 estimates.

| Intel macOS workload | Median | Median confidence interval |
|---|---:|---:|
| JSONL parse, 100k events | 162.311 ms | 147.442–197.634 ms |
| Independent replay, 100k events | 317.095 µs | 272.312–338.033 µs |
| Full check, 100k events | 8.966 ms | 8.529–9.354 ms |
| D03 reduction, 100k events | 19.656 ms | 19.147–20.382 ms |

All seven workloads retain 50 raw samples. The retry checkout is `5e1faad`;
its Rust/manifest source and input hashes match final production source
`e348f56` and the latest Linux record exactly.
[Intel samples and machine/compiler records](../bench-results/2026-11-04-lookup-macos-x86_64/),
[verified transfer](../bench-results/2026-11-04-lookup-macos-x86_64/TRANSFER.json).

## Mutation hardening

The initial expanded campaign completed all 560 mutations but failed the
zero-timeout gate. Three mutants caused nontermination: two repeatedly
reinserted zero FIFO lots, and one repeated the final no-progress pass.
[Complete initial failure evidence](../bench-results/2026-11-04-claim-boundaries-initial-linux-x86_64/MUTATION.md).
The gate was not relaxed and timeouts were not counted as kills.

FIFO consumption now removes one entry per iteration and returns any residual
lot after the loop. Certification passes are bounded by initial removable units
plus one; every accepted pass removes a unit and an actual no-progress pass
remains necessary to certify minimality. The existing FIFO regression also
checks a partially consumed buy lot funding two sells.

All three previously hanging mutations are now caught with zero timeouts:
[focused regression evidence](../bench-results/2026-11-04-claim-boundaries-linux-x86_64/mutation-timeout-regression/).

The broad campaign at source `111b1db` evaluates **561 mutations: 425 caught, 79 missed,
57 unviable, zero timeouts**. Every actual mutant uses locked workspace tests.

| Ratchet scope | Caught/viable | Missed | Unviable |
|---|---:|---:|---:|
| Legacy numeric/checker | 347/387 | 40 | 33 |
| Reducer | 78/117 | 39 | 24 |
| Fingerprints, a checker subset | 11/11 | 0 | 0 |

The legacy floor increases from 270/315 to 347/387. Adding reducer mutations
does not dilute that floor; reducer and fingerprint floors are independently
enforced. The full combined score is 425/504. Survivors remain disclosed,
including search branches that the current pinned-witness corpus does not
fully exercise. No equivalence is assumed; mutation scores do not prove the
absence of financial defects.
[Complete campaign, source snapshots and survivor list](../bench-results/2026-11-04-claim-boundaries-linux-x86_64/MUTATION.md),
[passing ratchets](../validation-results/claim-boundaries/mutation-ratchet.txt).

Final review found that an order name can reuse a symbol first interned as a
trade/claim string, breaking the lookup's numeric-sort assumption. Source
`e348f56` adds a numeric fallback and an end-to-end long-lived-order collision
regression. All four mutations of the lookup helper are caught with zero
timeouts: [focused schema record](../bench-results/2026-11-04-lookup-linux-x86_64/mutation-order-lookup/).

The broad campaign remains explicitly tied to workspace `111b1db`; it was not
rerun after the schema-only addition. All six broadly audited source files are
byte-identical in the final source:
[source comparison](../bench-results/2026-11-04-lookup-linux-x86_64/broad-mutation-source-match.txt).
Fresh native, platform and scale checks cover the final source separately.

## Limits and remaining scope

Budgets are cooperative: mandatory input validation, setup already in progress
and an admitted replay finish safely. Validation is separate from candidate
call counts. A zero budget may return the verified original trace; if no known
witness satisfies the unchanged lifecycle predicate, reduction returns an
explicit error. Limit exhaustion never claims minimality.

Checkpoint state is synthetic and conditional on the removed source history.
Bundle checksums prove integrity, not author authentication. This is a Mode T
output witness, not an engine-input reproducer. Minimality is relative to pinned
units, not a global event minimum. The whole-process 100-byte/event memory goal
remains unmet because original JSONL and column storage are retained.

Real engine adapters, multi-currency, shorting/margin, non-market orders,
corporate actions, streaming/mmap storage and a web UI remain outside Part 1.
