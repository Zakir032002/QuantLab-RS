# Claim-boundary corrections: measured Linux record

Source: `e348f56`, with exact crate/lockfile hashes in `source-sha256.txt`.
Compiler and machine: `rustc.txt` and `machine.json`. Runs started on 2026-11-04
(Asia/Calcutta). Rust 1.99.1, portable CPU target, system allocator, fat LTO,
codegen-units=1 and checked release arithmetic; no dependency changes.

```sh
RAYON_NUM_THREADS=5 cargo run -p qlrs-testkit --example scale_gate --release --locked
RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50
RAYON_NUM_THREADS=5 cargo run -p qlrs-testkit --example rich_gate --release --locked -- 250000 128 2000
GUNGRAUN_ALLOW_ASLR=yes RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench instructions --locked
python3 ci/instruction-ratchet.py bench-results/2026-11-04-lookup-linux-x86_64/instructions.txt
```

Criterion retains default warm-up and measurement settings and 50 raw samples
per workload, copied from `target/criterion/**/new`. Timed parse excludes input
buffer cloning. Replay is single-threaded. The fixture has 100,000 events and
20% fills. Shared-cloud wall times are observations, not hardware promises.
Mutation testing ran after timed performance workloads, without contention.

The rich gate uses seed 42, 250,000 ordinary orders, 3/4 instruments, sparse
claim intervals, interleaved partial fills and long-lived orders. All eleven
violating defects run on all three profiles, plus three large clean controls.
Each witness must have at most 128 observed events, complete within 2000 ms,
preserve the exact target, reproduce observed bytes and verify from a synthetic
checkpoint. Reducer call limit: 1024. Parse/preparation and witness serialization
are outside the reduction clock; initial checking and reducer setup are inside.

The legacy million-event D01/D03/D07 gate retains its six-event limit.
`column_bytes` estimates column capacities; `VmHWM` is process high-water memory
and includes retained JSONL. The whole-process 100-byte/event goal remains unmet.

Valgrind 3.24.0 / gungraun-runner 0.20.0 supplies deterministic Ir counts.
ASLR is enabled because this container disallows personality changes. The
unchanged 5% ratchet uses `ci/instruction-baseline.json`.
The initial measurement directory records the dynamic-range instruction
regression and its correction. This directory measures the order-name collision fallback. Mutation testing runs separately from timed performance workloads.

The broad 561-mutation record remains frozen at source 111b1db in the
claim-boundaries directory. The six broadly audited files are unchanged here.
Focused schema mutation evidence covers the new order lookup fallback.
