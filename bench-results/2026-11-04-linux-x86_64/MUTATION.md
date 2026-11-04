# Completed final numeric/checker mutation campaign

Frozen source baseline: 47ed446a6f3a50338b30693680184c623bb916b4.
The two source snapshots and mutation-sources.txt match the final crate files.

```sh
PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1 cargo mutants -p qlrs-num -p qlrs-check --test-workspace true --jobs 3 --no-shuffle --cargo-arg=--locked --output /workspace/setup/qlrs/mutation-final-final
```

All **347 generated mutations completed**: **270 caught, 45 missed, 32
unviable, zero timeouts**. The viable score is **270/315**. The measured score
increases from the initial 267/312 floor; ci/mutants-baseline.txt now records
the final campaign. Survivors are retained in mutation-missed.txt and are not
silently classified as equivalent. Improving coverage remains useful work.

The tool checks its unmutated baseline on the num/check packages; actual mutant
build/test phase argv contains both --workspace and --locked. The separate full
native suite also passes. Scope is num/check mutations tested by all workspace
tests; this does not establish reducer mutation coverage.

mutation-outcomes.json includes every scenario, phase result, exact Cargo argv,
completion time and successful unmutated baseline. mutation-mutants.json records
the full enumeration; mutation-console.txt and caught/missed/unviable/timeout
lists preserve raw results. cargo-mutants exits 2 for survivors; this is an
expected completed campaign, not a claim of a perfect mutation score.
Interrupted exploratory campaigns are not this final baseline.

To check the committed archive directly (the optional enumeration argument
accounts for the archive's prefixed filenames):

```sh
python3 ci/mutation-ratchet.py bench-results/2026-11-04-linux-x86_64/mutation-outcomes.json bench-results/2026-11-04-linux-x86_64/mutation-mutants.json
```
