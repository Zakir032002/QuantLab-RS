CP2.0 source is identified by the checkpoint commit containing these files.
Same machine and Rust compiler as ../before/machine.json and rustc.txt.

cargo nextest run --workspace --locked
cargo test --doc --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
scripts/known_failures.sh

Before/after throughput belongs to CP2.12, after the performance implementation.
This checkpoint reports correctness only.
