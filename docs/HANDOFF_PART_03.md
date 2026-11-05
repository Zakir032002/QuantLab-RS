# Handoff — Part 3 (Scale and Hardening)

Starting commit: `4f8bcbc0a5828eba10b8ba81bfc05de3971f36fc`.
Status: **CP3.0 blocked; Part 3 implementation has not started.**
Instructions are preserved verbatim in [PART_03_INSTRUCTIONS.md](PART_03_INSTRUCTIONS.md).

## Part 2 gate failure

CP3.0 task 3 explicitly requires:

> If any Part 2 gate fails or its evidence is missing: **stop**. Write the failure
> into `docs/HANDOFF_PART_03.md` under "Part 2 gate failure" and do nothing else.

Part 2's R4 dense-checker scaling criterion remains failed. Its committed
second-attempt samples record a 10k-event median of 2,206,738 ns and a
100k-event median of 26,998,455 ns: **12.234554×**, above the unchanged **12×**
limit. The separate 0.5-second absolute bound passes. Overall Part 2 acceptance
was already marked incomplete in [HANDOFF_PART_02.md](HANDOFF_PART_02.md).

Reapplying the existing ratchet to those exact samples exits 1. This is a
revalidation of saved measurements, **not a new benchmark or a third optimization
attempt**. Raw samples, checker output, command, exit code, source hashes and host
metadata are preserved under [evidence/part3/cp3.0](../evidence/part3/cp3.0/).
The two original attempts remain in [CP2.2](../bench/results/CP2.2/).

## Part 2 verification (CP3.0)

The starting handoff claims the following results. Historical passes are not
represented as new CP3.0 executions; further reruns stopped at the known R4
failure, as task 3 requires.

| Gate | Starting evidence | CP3.0 status |
|---|---|---|
| R4 dense-error absolute time and scaling | [original attempts](../bench/results/CP2.2/), [ratchet](../ci/r4-scaling.py) | **FAIL**, saved samples revalidated; scaling exceeds 12× |
| Native suite, scoped witnesses, limits, v0/v1 bundles and certificates | [82-test output](../bench/results/CP2.11/final/native-nextest.txt) | Previously recorded PASS; not rerun |
| Protected catalogue | [catalogue output](../bench/results/CP2.11/final/catalogue.txt) | Previously recorded PASS; not rerun |
| Golden manifest/example, float ban, dependency direction, traceability and test-count ratchets | [routine output](../bench/results/CP2.11/final/routine.txt), [final hosted quality](../validation-results/part2/hosted-ci-final.json) | Previously recorded PASS; not rerun |
| Determinism, worker counts and forced cache collisions | [CP2.10](../bench/results/CP2.10/), [synthetic gate](../bench/results/CP2.12/gate.jsonl) | Previously recorded PASS; not rerun |
| D01–D11 × S1–S9 synthetic gate, including S9 and real certificate candidates | [891-case table](../bench/results/CP2.12/GATE.md), [mandatory regressions](../bench/results/CP2.12/regressions.jsonl) | Previously recorded PASS; not rerun |
| Algebraic evaluator differential checks | [differential output](../bench/results/CP2.12/differential.txt) | Previously recorded PASS; not rerun |
| Separate numeric/checker, reducer and fingerprint mutation floors | [ratchet](../bench/results/CP2.11/updated-ratchet.txt), [raw/derived provenance](../bench/results/CP2.11/COMMANDS.md) | Previously recorded PASS; not rerun |
| Instruction ratchets | [raw counts](../bench/results/CP2.12/instructions.txt), [ratchet](../bench/results/CP2.12/instruction-ratchet.txt) | Previously recorded PASS; not rerun |
| Linux/Intel Mac quality, Miri, scale/instructions and Mac benchmarks | [observed five-job result](../validation-results/part2/hosted-ci-final.json) | Previously recorded PASS; not rerun |

CP3.0's rebase table, future-tool version inventory and pathological-input
baseline have not been started: those are tasks 4–6, after the blocking check.

## Checkpoint status

| Checkpoint | Status | Evidence | Notes |
|---|---|---|---|
| CP3.0 | **BLOCKED** | [revalidation.json](../evidence/part3/cp3.0/revalidation.json) | No passed checkpoint or tag claimed |
| CP3.1–CP3.15 | NOT STARTED | None | Blocked by the CP3.0 prerequisite |
| Gate S | NOT RUN | None | No `part3-gate-s` tag |

## Blockers

R4 already exhausted its two Part 2 optimization attempts. The existing
[repository rule](../AGENTS.md) says not to silently retry or relax it; Part 3's
CP3.0 explicitly stops on any failed Part 2 gate. Product code, verdicts,
fingerprints, protected fixtures and thresholds were not changed.

The next decision is whether to authorize reopening R4 for a new measured
optimization attempt with the original 12× scaling and 0.5-second absolute
criteria. Once that is authorized and passes, CP3.0 must rerun the remaining
checks before Part 3 implementation begins. There is no permission request
to waive or lower the threshold.
