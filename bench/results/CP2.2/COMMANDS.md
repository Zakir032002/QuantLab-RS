Development measurements on the same host/toolchain as CP2.0/before.
RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- 'pipeline/check_100k' --sample-size 50
cargo run --release --locked -p qlrs-testkit --example part2_metrics

These were uncommitted development snapshots during CP2.0..CP2.8. Exact full-tree
hashes were not captured for these exploratory runs; final before/after claims
must use the CP2.12 run with its exact source snapshot. Keep both dense attempts,
including the missed fixed scaling gate; docs/BLOCKERS.md records the stop.
