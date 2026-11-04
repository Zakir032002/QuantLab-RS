Same host/toolchain as CP2.0/before. Source is the containing CP2.9 commit.
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --locked -p qlrs-testkit --test part2
Original v0 writer reconstruction: fixtures/compat/README.md.
