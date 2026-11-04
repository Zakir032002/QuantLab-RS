Same host/toolchain as CP2.0/before/machine.json and rustc.txt.
QLRS_DIFFERENTIAL_CASES=10000 cargo nextest run --workspace --locked
QLRS_DIFFERENTIAL_CASES=10000 cargo test --locked -p qlrs-testkit --test differential_part2 -- --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --doc --workspace --locked
cargo test --locked -p qlrs-reduce --lib

The development gate predates the final S3/S9 schedule refinements. The final
891-cell gate and determinism/certificate outputs will be recorded at CP2.12.
R4 scaling remains a disclosed stopped CP2.2 gate, not a pass.
