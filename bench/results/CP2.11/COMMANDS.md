# CP2.11 commands and provenance

Tool: cargo-mutants 27.1.0, pinned Rust 1.99.1. Release metadata checked in
tool-releases.json. The source base is a877c7b plus the changes captured by
final/mutation-source-sha256.txt. Production mutation sources did not change
between the complete campaign and the focused rechecks.

The complete campaign used these environment settings:

```sh
export PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 RAYON_NUM_THREADS=1
export CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_OPT_LEVEL=0 CARGO_PROFILE_TEST_DEBUG=0
cargo mutants -p qlrs-num -p qlrs-check -p qlrs-reduce --copy-target true --test-workspace true --jobs 4 --no-shuffle --cargo-arg=--locked
```

The committed timeout configuration is multiplier 5, minimum 60 seconds. The
observed per-test timeout was 60 seconds. Debug/overflow assertions stayed on;
all workspace tests were selected. This build profile reduces mutation rebuild
cost only; normal native and release performance checks are retained separately.

final/ contains the complete 743-mutation enumeration, raw outcomes, campaign
console and individual logs/diffs in raw-logs.tar.gz. Its unmodified result is 545 caught, 121 missed,
74 unviable and 3 timeouts. The full campaign ran the then-current 81 tests.

scope-rerun/raw/ rechecks three exact checker sites with the new scope/status
regression (82 tests): finish at 854:45, witness_report at 894:25 and 895:25.
All three were caught. timeout-rerun/raw/ rechecks the three original reducer
timeouts serially, with the same 60-second cap: 1614:22 and 1615:17 survived,
1650:9 was caught. No timeout is counted as a kill. Actual Cargo commands,
mutation descriptions and test outcomes are preserved in each raw result.
Only the baseline build/test may be package-scoped by cargo-mutants; every
mutated test invocation uses --workspace. Both rechecks used --jobs 1.

The auditable derived report retains the full enumeration and explicitly records
the six replacements and raw-file hashes; it is not a new raw campaign:

```sh
python3 ci/merge-mutation-rechecks.py bench/results/CP2.11/final/outcomes.json bench/results/CP2.11/final/mutants.json bench/results/CP2.11/effective-outcomes.json bench/results/CP2.11/scope-rerun/raw/outcomes.json bench/results/CP2.11/timeout-rerun/raw/outcomes.json
python3 ci/mutation-ratchet.py bench/results/CP2.11/effective-outcomes.json bench/results/CP2.11/final/mutants.json
python3 ci/mutation-diff.py bench/results/CP2.11/scope-rerun/raw/outcomes.json
```

ratchet.txt checks the original separate floors; updated-ratchet.txt checks the
new nondecreasing floors. helper-kills/ preserves every evaluate_batch and
remove_closure mutation: five caught, two unviable, none missed or timed out.
docs/MUTANTS_PART_02.md individually triages all 120 remaining survivors.

The incomplete attempt-optimized-tests/ and attempt-before-cache-guard/ records
are excluded from all scores. The public cache-bounds suspicion was refuted;
the incorrect assertion and corrected reasoning are retained explicitly.
ADR-0015 records the real missing-Git-path build invalidation correction and
the defensive private-batch precondition. No public cache bug is claimed.

Normal-profile verification, including 82 passing tests, clippy, doctests,
protected catalogue and 400000 independent numeric comparisons, is in final/.
