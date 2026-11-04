# Focused regression for the three former mutation timeouts

Source: 111b1dbf62cd1b4f28fa10321adc1dc4bcf4f3da.
All three selected mutations are caught; zero missed/unviable/timeouts.
This focused run supplements the complete campaign; it is not a full-scope gate.

```sh
PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1 RAYON_NUM_THREADS=1 cargo mutants -p qlrs-reduce --re '(:84:38: replace > with (==|>=) in UnitGraph::build_with_checkpoint|:833:41: delete ! in reduce_with_limits)$' --copy-target true --test-workspace true --jobs 3 --timeout 60 --no-shuffle --cargo-arg=--locked --output /workspace/setup/qlrs/mutation-timeout-regression
```
