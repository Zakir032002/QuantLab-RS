# Focused order lookup mutation campaign

Source: e348f562128c59c337590869544afd34aa91bfdf.
All four generated mutations of TraceStore::order_index are caught; zero
missed/unviable/timeouts. This includes the fallback equality condition.

```sh
PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1 RAYON_NUM_THREADS=1 cargo mutants -p qlrs-schema --re 'TraceStore::order_index' --copy-target true --test-workspace true --jobs 2 --timeout 60 --no-shuffle --cargo-arg=--locked --output /workspace/setup/qlrs/mutation-order-lookup
```

The broad 561-mutation campaign remains frozen at 111b1db. Its six audited
numeric/checker/reducer files are unchanged by this later schema correction;
its full workspace snapshot and actual outcomes remain separate. The added
order/trade-name collision regression and fresh native/platform checks validate
the final source. This focused run is not presented as a new broad campaign.
