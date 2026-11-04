# Commands and provenance

Linux source: a877c7b plus the recorded patch (internal cache-domain guard,
build invalidation fix and prefix harness). source-sha256.txt is the exact
pre-measurement snapshot; LINKED_SOURCE_EQUIVALENCE.json verifies runtime and
benchmark inputs match current source. The reducer test module was corrected
afterwards; it is excluded from benchmark executables.
Machine/compiler: machine.json, rustc.txt. No concurrent mutation campaign ran
during Criterion or the gate timings.

    cargo run --release --locked -p qlrs-testkit --example part2_gate
    cargo run --release --locked -p qlrs-testkit --example part2_regressions
    RAYON_NUM_THREADS=5 CARGO_TARGET_DIR=/tmp/qlrs-v0-target cargo bench --manifest-path /tmp/qlrs-v0-source/Cargo.toml -p qlrs-bench --bench throughput --locked -- 'pipeline/check_100k|reduce_100k' --sample-size 50
    RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- 'pipeline/check_100k|reduce_100k|prefix_residual' --sample-size 50
    GUNGRAUN_ALLOW_ASLR=yes cargo bench -p qlrs-bench --bench instructions --locked
    python3 ci/instruction-ratchet.py bench/results/CP2.12/instructions.txt
    QLRS_DIFFERENTIAL_CASES=10000 cargo test --locked -p qlrs-testkit --test differential_part2 -- --nocapture

Before source is an exact archive of 281fa154ac3e2cae4a3f2155d025b91c57b30e97.
Every archived Rust/manifest/lock file was checked against that commit, then built
and measured on this same host with the same pinned compiler and release flags.
This reconstruction is labeled, not backdated as a CP2.0 execution.

Criterion console change percentages can compare prior local cache baselines;
use the committed before/after estimates.json medians for reported comparisons.
Mac after samples came from the actual successful a877c7b hosted job; transfer
ZIP and per-file hashes, source commit and input hashes were verified. The retained
Intel before record has identical Rust/manifest/lock hashes to 281fa154. Both
report i7-8700B / four CPUs / macOS 15.7.9; physical host identity is not known.
No M1 or samply result is claimed. Kani remains optional and unexecuted locally.
