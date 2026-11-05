# Part 2 measurements

Raw data, commands, compiler and actual machine metadata are committed under
[CP2.12](../bench/results/CP2.12/COMMANDS.md). These are observed shared-host
measurements, not universal speed guarantees. Before is source 281fa154; after
core is a877c7b plus the recorded internal cache-domain guard; exact measured
sources are reconstructed by measurement.patch and checked by linked-source
equivalence. The Mac final-source samples come from the successful 4941acd job;
the earlier a877c7b development run remains in macos/after. The Mac before record
is retained from Part 1 with matching source hashes. Same declared Intel runner hardware,
not proven same physical host. Unchanged numeric benchmarks also vary on these
shared runners; do not attribute every Mac difference solely to code changes.

50-sample Criterion median estimates:

| Workload | Before | After | Observed reduction |
|---|---:|---:|---:|
| Linux pipeline/check_100k | 2.995 ms | 2.202 ms | 26.5% |
| Linux reduce_100k | 6.289 ms | 4.651 ms | 26.0% |
| Intel macOS pipeline/check_100k | 8.966 ms | 4.376 ms | 51.2% |
| Intel macOS reduce_100k | 19.656 ms | 11.906 ms | 39.4% |

The O(1) checked prefix query measured 7.18 ns (Linux median); it has no
Part 1 equivalent. Exact samples and estimates are in after/criterion.

The [891-cell case table](../bench/results/CP2.12/GATE.md) reports every
shape/defect/size/seed separately, including kept events, exhaustive minimum,
full-check slots, fast probes, certificate tests, wall time and log bytes.
All cells pass. 564 cases perform real removable-root certificate tests; the
others explicitly report zero/vacuous. All 891 small-window results equal their
exhaustive minimum. The largest 100k-budget reduction took 148.109126 ms; actual
inputs span 100001–200014 events. Full candidate slots never exceed 10 in the
matrix. Mandatory validations are separate and no wall-time budget fired.

The extra sparse case has 10211 input events and keeps 104, with 26 actual
certificate tests and 28 reserved full slots. Its required window legitimately
exceeds 32. Explicit R1a/R1b/R1c/R1d rows are in regressions.jsonl.

Linux instruction counts are 1297 decimal parse, 125 multiply/rescale and 33519
for the 1000-event replay. All original instruction ratchets pass unchanged.
The fresh differential run covers 10000 cases per invariant per shape (180000
closed subsets); see differential.txt. Nightly configuration uses 100000.

R4's separate dense-error scaling criterion remains a miss: median ratio
12.2346 exceeds 12, although the 100k checker takes 26.998455 ms, below 0.5 s.
Both attempts and the stopped-checkpoint explanation remain in CP2.2 and
[BLOCKERS.md](BLOCKERS.md). No threshold was changed and no third attempt is
claimed. Kani and samply profiles are not measured locally. Current M1 hardware
was unavailable; the existing ADR-0009 Intel fallback is labeled explicitly.

Final-source Intel samples and verified transfer hashes are in macos/final-source.
Intel replay was 0.317 ms before and 0.258 ms on final source. The retained
a877c7b development run measured 0.347 ms (an observed regression), with checker
4.649 ms and reducer 11.764 ms. All runs remain committed; shared-runner effects
are not isolated and the fastest observation is not substituted for final source.
