# Handoff — Part 3 (Scale and Hardening)

Starting commit: `4f8bcbc0a5828eba10b8ba81bfc05de3971f36fc`.
Status: **CP3.0 resumed and verified after user authorization; implementation in progress.**
Instructions are preserved verbatim in [PART_03_INSTRUCTIONS.md](PART_03_INSTRUCTIONS.md).

## Resumed prerequisite verification

The user authorized reopening R4 and building Part 3. A fresh run on unchanged
source passed the original R4 limits (ratio 11.105456). The original failures
remain historical evidence; this is no claimed algorithm improvement.
Fresh verification passes 82 native tests plus one doctest, all 891 synthetic
gate cells, the catalogue, golden checks, float/dependency guards, mutation
floors, test-count ratchet and full 1/2/8-worker determinism. Commands, host,
toolchain and outputs are in evidence/part3/cp3.0 and r4-reopened/before.
The rebase is [PART3_REBASE.md](PART3_REBASE.md); current behaviour and its
executed probes are [PART3_CURRENT_BEHAVIOUR.md](PART3_CURRENT_BEHAVIOUR.md).
Versions were rechecked against the official registry; no dependency bump was made.

## Historical Part 2 gate failure

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
| CP3.0 | **RESUMED / VERIFIED** | [revalidation.json](../evidence/part3/cp3.0/revalidation.json) | Fresh verification: resumed-metadata.json; no tag |
| CP3.1–CP3.15 | NOT STARTED | None | Blocked by the CP3.0 prerequisite |
| Gate S | NOT RUN | None | No `part3-gate-s` tag |

## Initial blockers (superseded by user authorization)

R4 already exhausted its two Part 2 optimization attempts. The existing
[repository rule](../AGENTS.md) says not to silently retry or relax it; Part 3's
CP3.0 explicitly stops on any failed Part 2 gate. Product code, verdicts,
fingerprints, protected fixtures and thresholds were not changed.

The next decision is whether to authorize reopening R4 for a new measured
optimization attempt with the original 12× scaling and 0.5-second absolute
criteria. Once that is authorized and passes, CP3.0 must rerun the remaining
checks before Part 3 implementation begins. There is no permission request
to waive or lower the threshold.

## Current work and approvals

The R4 stop was explicitly reopened by the user. CP3.1 baseline and CP3.2
streaming generator are next. R9 review for default parser limits is pending;
[the concrete verdict differences](PART3_VERDICT_REVIEW.md) are recorded while
independent baseline/generator work continues. No Part 3 default verdict change
has been enabled without that approval. No Gate S pass or tag is claimed.

## CP3.1–CP3.2 progress

CP3.1 before baseline committed as ce6480b. CP3.2 reuses the old generator
logic through generic writers; the new streaming CLI generated exactly 10M
events with 2488 KiB peak RSS (raw GNU time: evidence/part3/gen/s10m).
This passes the 64 MiB Linux goal. The original 100k generator byte commitment
passes unchanged. All 85 native tests pass, with no skips; catalogue and goldens
pass. Two 1M generations match both trace bytes and metadata. ARM confirmation
is pending, not counted as a pass. Source is ce6480b plus gen/source.patch, with
exact source hashes in each measurement metadata. No Gate S pass is claimed.

## Resource checks and fuzz foundation

Opt-in resource limits, typed CLI errors, bundle confinement checks and atomic
bundle staging are implemented. All existing defaults remain pending the R9
review in PART3_VERDICT_REVIEW. Boundary and CLI outputs are in
`evidence/part3/limits`; the native suite, protected catalogue and goldens pass.
This is implementation progress, not CP3.3 completion while review is pending.

F01/F02/F05/F06 each completed a 60-second smoke campaign with final statistics,
no crashes, and minimized corpora. Stable replay uses the same oracle functions.
Logs are in `evidence/part3/fuzz-smoke` and `fuzz-cmin`. These smoke runs are not
claimed as the one-hour Gate S campaigns. The malformed dictionary setup run is
retained and excluded. Local ASan used detect_leaks=0 because this sandbox does
not permit LeakSanitizer's ptrace scan. Hosted fuzz smoke is pending.

The exact before source ce6480b has now been measured on both actual macos-15
ARM and ubuntu-24.04-arm. Both jobs succeeded; transferred outputs include
verified per-file and source hashes under `baseline-before/<runner>`. Those are
before results and do not establish the later scale or determinism gates.

## Latest implemented state

All eleven applicable targets build and have zero-crash smoke evidence. F03
also completed 20 minutes (881439 executions); this is not Gate S CPU-hour
evidence. Native tests: 109, floor raised to 109. All corpora were minimized
before their commits, with at most 2000 files each. The sole new target failure
was P3-ORACLE-001, an invalid later-anchor request; minimized regression and
correction are separate commits 34ae0b5 and b5a126e, and the regression passes.
No production invariant or fingerprint policy changed for that correction.

File ingestion retains offsets and block digests, not full source bytes. The
original CP3.1 1M input now peaks at 153760 KiB versus 332436 KiB before; raw
streaming/verify-s1m and baseline-before/verify-s1m. The 10M process completes,
but 1561384 KiB exceeds its RSS gate; compaction work is active. Removing the
unused Unit.first_event field lowered the 1M development measurement to 149220
KiB (hot-path/unit-column). Worktree source hashes are recorded with each run;
these intermediate measurements are not represented as final Gate S baselines.

Bundle reads now use safe rustix directory-relative descriptors and NOFOLLOW;
file/root replacement race tests pass. Bundle staging also uses atomic
NOREPLACE rename. Kani 0.68.0 is installed with its own pinned toolchain; proofs
have not yet run. Main-branch push of 2ce5ff9 was rejected by automatic approval
review pending explicit permission. Hosted validation will use a new review
branch as the safer alternative; main approval remains pending. No Gate S tag.
