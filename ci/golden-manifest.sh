#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
qlrs_actual=$(mktemp)
trap 'rm -f "$qlrs_actual"' EXIT
cargo run --quiet -p qlrs-io --example hash --locked -- fixtures/golden > "$qlrs_actual"
diff -u fixtures/golden/MANIFEST.blake3 "$qlrs_actual"
echo 'golden manifest: PASS'
