#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked
cargo deny check
cargo audit
./ci/check-deps.sh
./ci/float-ban.sh
./ci/golden-manifest.sh
./ci/golden-report.sh
python3 ci/traceability.py
python3 ci/test-count.py
./ci/determinism.sh
