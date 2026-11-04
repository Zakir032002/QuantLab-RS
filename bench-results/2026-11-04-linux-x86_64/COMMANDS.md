# Reproduction commands and measurement scope

Machine and compiler: machine.txt and rustc.txt. All runs use Rust 1.99.1,
release/bench fat LTO, codegen-units=1, overflow checks enabled, default portable
CPU target and the system allocator. JSONL workloads use seed 42; source digests
for scale cases are in scale-gate.txt. The synthetic lifecycle is 20% fill events.

From the repository root, with the pinned toolchain activated:

```sh
RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50
GUNGRAUN_ALLOW_ASLR=yes cargo bench -p qlrs-bench --bench instructions --locked
RAYON_NUM_THREADS=5 cargo run -p qlrs-testkit --example scale_gate --release --locked
rustc -Vv
uname -a
lscpu
cat /etc/os-release
cat /sys/fs/cgroup/cpu.max
```

Criterion retains its default warm-up and measurement configuration, with 50
samples requested. Raw estimates/sample JSON is copied from target/criterion.
The timings are measurements from a shared Linux cloud instance, not promises
for an M-series core. Miri and mutation runs were separate from the final timed
Criterion workload. Replay is single-threaded. Parallel JSON parsing uses five
Rayon workers; input-buffer cloning is outside the Criterion timed parse region.

Gungraun uses Valgrind 3.24.0 and runner 0.20.0. The local wrapper supplies the
workspace Valgrind library/debug paths because the runner clears child env.
ASLR remains enabled because container policy forbids setarch personality
changes. Ir counts are unaffected by wall-clock scheduling. The instruction baseline is recorded in ci/instruction-baseline.json. The final
run measures decimal_parse=1297, mul_rescale=125 and replay_1000=35063; decimal
parsing remains within the 5% gate relative to its initial 1288-instruction
baseline (also shown as the previous-count column in the raw output).

scale-gate-before-compaction.txt records the pre-compaction run;
scale-gate-during-builds.txt records an explicitly contended intermediate run.
scale-gate.txt is the final functional scale gate. Report column_bytes as an
estimate of retained column capacities, separate from original source bytes,
line ranges, temporary graph storage and measured Linux process VmHWM.
The whole-process 100-byte/event engineering target is not attained.

Mutation command and exact completion results are recorded separately in the
mutation files after that campaign completes. Interrupted exploratory campaigns
are not reported as the completed mutation baseline.

The overflow-check experiment changes only the bench profile setting:

```sh
CARGO_PROFILE_BENCH_OVERFLOW_CHECKS=false GUNGRAUN_ALLOW_ASLR=yes cargo bench -p qlrs-bench --bench instructions --locked
```

`instructions-overflow-off.txt` records the same Ir counts as the default
enabled build for all three workloads. Keep overflow checks enabled. This is
an instruction experiment, not a wall-clock comparison across all money paths.
`source-sha256.txt` commits the measured crate sources and lockfile;
`input-digests.txt` commits the generated 100k-event pipeline fixtures.
