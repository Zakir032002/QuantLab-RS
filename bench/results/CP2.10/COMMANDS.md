Same host/toolchain as CP2.0/before. Source is the containing CP2.10 commit.
cargo nextest run --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
./ci/determinism.sh
The determinism script compares reports and every bundle byte with 1, 2 and 8
workers, including all eleven rich defects and call-limited output; every
certificate is rechecked. Raw full-matrix timings belong to CP2.12.
