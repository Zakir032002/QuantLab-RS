# Part 1 handoff

The synthetic counterexample engine is implemented end to end. Local acceptance
checks and both synthetic gates pass. Hosted Linux/macOS quality, Miri, scale,
instruction and macOS benchmark jobs all passed for evidence commit
`7ff05069f19358e646687e41cf5d6b80afaaec6e`. The final raw-record/publisher follow-up
preserves the tested Rust source hashes. Engineering goals remain separate from
functional gates.

## 1. Delivered implementation

The repository was initially empty. This is an integrated implementation, not
nine fabricated historical green checkpoint tags. The source baseline is `47ed446a6f3a50338b30693680184c623bb916b4`
(`cp1.7: implement the Part 1 synthetic counterexample engine`). The measured
evidence/handoff follows in cp1.8 commit `7ff05069f19358e646687e41cf5d6b80afaaec6e`;
source hashes independently commit the code used in both platforms' runs.

| Checkpoint | Delivered behavior |
|---|---|
| CP1.0 | Ten-crate workspace, pinned compiler/stack/lockfile, strict lints, dependency direction, licenses/advisories, protected golden manifest, traceability, test/mutation ratchets and Linux/macOS CI. A deliberate forbidden dependency fails the guard and was reverted. |
| CP1.1 | Allocation-free decimal parsing, private per-value i128 fixed point, checked math, cold safe U256 fallback, constant-divisor rescaling, four signed rounding modes and independent big-integer properties. |
| CP1.2 | Strict JSONL boundary, borrowed strings with escaped-string fallback, dense interner, per-kind SoA columns, million-event parsing and verbatim observed-line writing. |
| CP1.3 | Prepared independent cash/position deltas; claims never drive expected state. C01 computes exactly 7990.00. Replay performs zero allocations in the instrumented loop. |
| CP1.4 | I1 local cash, I2 cumulative position, data-driven I3 lifecycle, preconditions, numeric limitations, verdict precedence, partial mode and exact exit codes. |
| CP1.5 | Exact instance keys, canonical BLAKE3 class fingerprint with fixed vector, seeded generator, protected D01–D14 catalogue and clean/random injection tests. Gate A-synthetic passes locally. |
| CP1.6 | FIFO inventory dependencies, u32 CSR graphs, bitset closures, deterministic parallel ddmin, pinned evidence, prefix snapshots, synthetic checkpoints, cached predicate logs and closure-1-minimal/full-replay checks. Early Gate B passes on D01/D03/D07. |
| CP1.7 | verify/reduce/bundle verify, hashed payloads and detached manifest commitment, strict paths, byte-tamper checks, checkpoint ancestry, repeat reduction and reproduction instructions. One/eight-thread bytes match. |
| CP1.8 | Criterion samples/intervals, Linux instruction and mutation baselines, overflow-check experiment, raw machine/source commitments and this handoff. |

The [complete setup and quality run](../validation-results/cloud-setup-repeat.txt)
passes **42 native tests, zero skips**, plus one doctest, fmt, clippy, deny,
audit, dependency direction, float ban, golden integrity/report, traceability,
test-count and thread determinism. The test-count floor is 42.
[Dependency negative test](../validation-results/dependency-negative-test.txt),
[traceability](TRACE.md), [stack](STACK.md), [workflows](../.github/workflows/).

## 2. Not completed or outside scope

Hosted success is observed directly in the [complete workflow](https://github.com/Zakir032002/QuantLab-RS/actions/runs/41836096568)
and [saved job statuses](../validation-results/hosted-ci-current.json). Fresh-task
restoration of the cloud snapshot has not been independently tested; saving the
draft does not publish it.

The whole-process memory engineering goal is unmet. The hosted Mac parse
measurement is also below the original 300 MB/s engineering goal with five
workers on its three virtual CPUs; the complete measured configuration is below. Optimized I2/I3 algebraic
admission remains future work; complete checking is the fallback. Real engine
adapters, engine-input reproduction, multiple currencies, margin/shorts,
non-market orders, corporate actions, UI, fuzz campaigns and PGO are later parts.

## 3. Measured numbers

Shared Linux x86_64, AMD EPYC 9V74, five visible CPUs, cgroup quota
400000/100000; Rust 1.99.1, portable CPU target, system allocator, fat LTO and
overflow checks enabled. No M-series extrapolation. Inputs contain 20% fill
events. Replay is single-threaded; parsing uses five Rayon workers. Input buffer
cloning is outside the timed parse region. Criterion uses default warm-up and
measurement settings, with 50 samples.

| Workload | Median | Stored median confidence interval |
|---|---:|---:|
| Decimal parse | 74.097 ns | 73.143–74.898 ns |
| i128 multiply/rescale | 11.823 ns | 11.731–11.983 ns |
| Wide multiply/rescale | 449.476 ns | 444.201–458.943 ns |
| JSONL parse, 100k events | 49.797 ms | 49.056–50.276 ms |
| Independent replay, 100k events | 369.029 µs | 363.144–375.371 µs |
| Full check, 100k events | 1.833 ms | 1.810–1.864 ms |
| D03 reduction, 100k events | 5.749 ms | 5.708–5.938 ms |

Raw [console](../bench-results/2026-11-04-linux-x86_64/criterion.txt),
[estimates and samples](../bench-results/2026-11-04-linux-x86_64/criterion/),
[commands/scope](../bench-results/2026-11-04-linux-x86_64/COMMANDS.md),
[machine](../bench-results/2026-11-04-linux-x86_64/machine.txt),
[compiler](../bench-results/2026-11-04-linux-x86_64/rustc.txt),
[source commitments](../bench-results/2026-11-04-linux-x86_64/source-sha256.txt),
[input digests](../bench-results/2026-11-04-linux-x86_64/input-digests.txt).
Console summary estimates differ from this table, which reads the stored medians.

Linux Valgrind Ir counts: decimal parse **1297**, multiply/rescale **125**,
replay of 1000 events **35063**. The 5% instruction ratchet passes. Disabling
bench overflow checks gives the same counts for these workloads, so checks
remain enabled. [Default counts](../bench-results/2026-11-04-linux-x86_64/instructions.txt),
[overflow-off experiment](../bench-results/2026-11-04-linux-x86_64/instructions-overflow-off.txt),
[ratchet](../validation-results/instruction-ratchet.txt).

The scale run retains approximately **99.2 bytes/event in columns**. This is a
capacity estimate including the interner, not RSS. Process high-water memory
across the three scale cases is **341096 kB**, approximately **333 MiB**.
Original JSONL alone occupies approximately 178.6 MB per case. Retained source,
line ranges, graphs and temporaries explain why the whole-process 100-byte/event
goal is unmet. Raw memory/column counts appear in the scale log below.

macOS measurements use a hosted **Apple M1 (Virtual)**, three CPUs, 7 GiB RAM,
macOS 15.8.2 and the same Rust 1.99.1, source files and input digests as Linux.
The recorded parse run uses five Rayon workers; replay is single-threaded.
These are shared virtual-runner observations.

| macOS workload | Median | Stored median confidence interval |
|---|---:|---:|
| Decimal parse | 104.818 ns | 94.640–111.839 ns |
| i128 multiply/rescale | 6.211 ns | 5.703–6.464 ns |
| Wide multiply/rescale | 656.373 ns | 644.288–710.496 ns |
| Full check, 100k events | 1.874 ms | 1.713–2.010 ms |
| JSONL parse, 100k events | 65.270 ms | 61.962–74.035 ms |
| Independent replay, 100k events | 296.720 µs | 286.132–306.336 µs |
| D03 reduction, 100k events | 5.193 ms | 4.907–5.429 ms |

[Raw macOS estimates and all samples](../bench-results/2026-11-04-macos-arm64/criterion/),
[machine](../bench-results/2026-11-04-macos-arm64/machine.txt),
[compiler](../bench-results/2026-11-04-macos-arm64/rustc.txt),
[commands](../bench-results/2026-11-04-macos-arm64/COMMANDS.md),
[input digests](../bench-results/2026-11-04-macos-arm64/input-digests.txt),
[original provenance](../bench-results/2026-11-04-macos-arm64/PROVENANCE.json),
[verified public transfer](../bench-results/2026-11-04-macos-arm64/TRANSFER.json).
Every raw numeric file matches the original artifact's manifest. Required
CPU/OS/compiler fields are retained; hostname and build-path console are omitted
from the public subset. The original complete artifact remains on its workflow.

## 4. Decisions and deviations

- [ADR-0001](adr/0001-numeric-representation.md): checked i128/U256 math. Stable
  carrying_mul compiles on the pinned compiler; an unmeasured rewrite is deferred.
  [Compile probe](../validation-results/carrying-mul-probe.txt).
- [ADR-0002](adr/0002-event-layout.md) and [ADR-0004](adr/0004-registry-verification-and-safe-input.md):
  safe buffered evidence rather than unsafe mmap, with the memory cost disclosed.
  Official crates.io metadata/checksums/source replace inaccessible docs.rs
  verification under the user's authorization to continue. TLS/proxy checks stay on.
- [ADR-0003](adr/0003-reduction-predicate.md): instance-preserving closure reduction,
  sparse I1 evaluator, full fallback and final cross-check. Fallible APIs return
  Result rather than hiding failures.
- [ADR-0005](adr/0005-allocation-instrumentation.md): the sole unsafe exception is
  an isolated test allocator forwarding unchanged pointers/layouts to System;
  production and money crates forbid unsafe. The exception passes Miri.
- [ADR-0006](adr/0006-hardening-and-validation-scope.md): round combined signed
  notional and fee once per fill, conservative missing-metadata handling,
  checkpoint ancestry, overflow checks retained and integrated delivery.

Miri passes five numeric edge tests, the combined-fee regression and allocation
instrumentation. Two 100000-iteration stress loops are filtered in Miri after a
prohibitively slow interpreter run; their full native volume is unchanged. This
is bounded interpreter coverage, not the entire high-volume property suite.
Raw [edges](../validation-results/miri-numeric-edges.txt),
[rounding](../validation-results/miri-combined-rounding.txt),
[allocation](../validation-results/miri-allocation.txt).

- [ADR-0007](adr/0007-cross-platform-benchmark-evidence.md): actual hosted macOS
  benchmarks, complete raw numeric records, verified commitments and bounded
  public metadata records.

## 5. Known risks and mutation coverage

The completed numeric/checker mutation campaign runs all workspace tests for
each viable mutant. Exact tested source snapshots, survivors, complete outcomes
and the measured ratchet are in
[MUTATION.md](../bench-results/2026-11-04-linux-x86_64/MUTATION.md).
Survivors are disclosed rather than classified as equivalent without proof.
The ratchet does not prove the absence of financial defects. The final campaign
completes 347 mutants: 270 caught, 45 missed, 32 unviable and zero timeouts; the viable score is 270/315. Mutation scope does not include
the reducer in Part 1.

Checkpoints commit to expected state and the original source digest; without the
source they do not independently prove removed history. Bundle hashes establish
integrity, not author authentication. Relations are triage hints, not diagnoses.
[BLOCKERS.md](BLOCKERS.md) records platform and memory limitations.

## 6. Synthetic gate results

Gate A passes: each defect at 1000 random injection points; 1000 generated clean
traces have no false violations. C01–C12 are CONSISTENT, including zero cash/fees,
flat-after-close, fractional 18-decimal quantity, sub-currency commissions and
all declared rounding modes. A separate clean million-event trace is CONSISTENT:
[raw verdict and digest](../validation-results/clean-million.txt).
The CLI is independently checked against the protected TOML catalogue in the
setup log. Expected statuses and tolerances were not weakened.

| Defect | Required and observed result |
|---|---|
| D01 double fee | VIOLATION I1 |
| D02 omitted fee | VIOLATION I1 |
| D03 one-share notional | VIOLATION I1 |
| D04 sell debit | VIOLATION I1 |
| D05 double cash fill | VIOLATION I1 |
| D06 missing partial position | VIOLATION I2 |
| D07 fill after cancel | VIOLATION I3 |
| D08 overfill | VIOLATION I3 |
| D09 fill before acceptance | VIOLATION I3 |
| D10 claim ×10 | VIOLATION I1 |
| D11 duplicate trade | VIOLATION I3 |
| D12 foreign fee currency | UNSUPPORTED |
| D13 short position | UNSUPPORTED |
| D14 inexact Exact cash | INCONCLUSIVE |

Early Gate B passes with seed 42 and injection at order 140000:

| Case | Input events | Retained observed events | Reduction | Predicate entries |
|---|---:|---:|---:|---:|
| D01 | 1000000 | 6 | 74.025 ms | 2 |
| D03 | 1000000 | 6 | 76.069 ms | 2 |
| D07 | 1000001 | 6 | 74.390 ms | 2 |

Each case asserts exact target replay, verbatim observed lines, a labelled
synthetic checkpoint and closure-1-minimality relative to pins. D07 inserts one
extra fill. This is output-witness reduction, not engine-input reproduction or
a global minimum. Times start after parse/prepare and include reducer setup and
final checks. [Raw scale evidence/digests](../bench-results/2026-11-04-linux-x86_64/scale-gate.txt).

Numeric properties run 100000 cases each, with boundary/signed values and all
rounding modes against num-bigint, including single-round notional-plus-fee.
The supplementary Python integer oracle passes 400000 comparisons:
[raw output](../validation-results/python-numeric-oracle.txt). Random candidate
masks compare sparse/full admission. Integration coverage includes exit statuses,
malformed line numbers, every-file tampering, idempotence and repeat bundling.

## 7. Stable interfaces for Part 2

Keep TraceStore, per-kind columns, dense IDs, delta_cash and AliveMask stable.
Call Ledger::prepare once. Ledger::replay returns Result<ExpectedState,
ReplayError>; replay_into reuses allocation-free state. Checker::check returns
the complete Report; evaluate retains the convenience entry point. Findings
carry InstanceKey, class_fp, residual and window.

Predicate::test returns a fallible Outcome; preserve errors separately from
rejection. UnitGraph exposes CSR dependencies, dependent closures and
prerequisites. reduce exposes the target, alive mask, checkpoint, complete
predicate log and closure-minimality result. Keep SyntheticCheckpoint,
qlrs-trace/v0, qlrs-bundle/v0, fingerprint bytes and catalogue format stable.
Changes require an ADR; protected expectations require human review.
[SPEC](SPEC.md) and [TRACE](TRACE.md) define the contracts.

## 8. Delivery and human actions

No service or account setup is needed. The tested full install script and startup
instructions are saved in the cloud configuration draft. Review and save the
changes in environment settings, then publish to activate the configuration and
snapshot. Fresh-task restoration has not been claimed.

Implementation baseline: `47ed446a6f3a50338b30693680184c623bb916b4`.
The implementation baseline is pushed to `Zakir032002/QuantLab-RS` on main.
The final evidence commit follows that source baseline. Hosted results are
recorded in [initial statuses](../validation-results/hosted-ci-status.json) and
[complete evidence-commit statuses](../validation-results/hosted-ci-current.json)
with observation times and workflow URLs. The integrated delivery does not invent
separate historical checkpoint tags. The final record commit retains the exact
Rust sources that passed those hosted jobs. Tag `cp1.8` identifies the integrated
Part 1 delivery, including both platforms' raw measurements and disclosed limits.
