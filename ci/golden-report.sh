#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
qlrs_actual=$(mktemp)
trap 'rm -f "$qlrs_actual"' EXIT
cargo run --quiet -p qlrs-cli --locked -- verify fixtures/clean/C01-worked-example.jsonl > "$qlrs_actual"
diff -u fixtures/golden/C01-report.json "$qlrs_actual"
echo 'independent worked-example golden: PASS'
