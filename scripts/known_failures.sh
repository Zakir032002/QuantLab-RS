#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Stage A already exists: run active regressions as well as any ignored cases.
cargo test --locked -p qlrs-testkit --test part2 -- --include-ignored
