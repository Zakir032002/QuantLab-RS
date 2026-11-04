# Initial expanded campaign: failed zero-timeout gate

Frozen production/test source: 956790b046bfca81560bea9b268cd4cef0a5a783.

```sh
PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1 RAYON_NUM_THREADS=1 cargo mutants -p qlrs-num -p qlrs-check -p qlrs-reduce --test-workspace true --jobs 4 --timeout 60 --no-shuffle --cargo-arg=--locked --output /workspace/setup/qlrs/mutation-claim-boundaries
```

All 560 attempted: 422 caught, 78 missed, 57 unviable, **3 timeouts**.
The unmodified baseline passed. This is failed evidence, not the passing
mutation baseline, and timeouts are not counted as kills.

Two mutants changed `remaining > 0` so FIFO consumption repeatedly reinserted
zero lots. A third negated the final no-progress certificate condition and
repeated search until per-call deadlines made the test binary time out.
The raw timeout list, three complete logs, enumeration, outcomes and exact
mutated-source snapshots are retained here.

The corrected source reinserts residual lots after the consumption loop and
bounds certification passes by initial removable units plus one. Its independent
full rerun and measured floors are in the final claim-boundaries result folder.
