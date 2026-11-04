#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
qlrs_temp=$(mktemp -d)
trap 'rm -rf "$qlrs_temp"' EXIT
cargo build -p qlrs-cli --locked
cargo run -p qlrs-testkit --example generate --locked -- 2000 "$qlrs_temp/input.jsonl" D03 1400
for threads in 1 8; do
    set +e
    RAYON_NUM_THREADS=$threads target/debug/qlrs verify "$qlrs_temp/input.jsonl" > "$qlrs_temp/verify-$threads.json"
    qlrs_code=$?
    set -e
    test "$qlrs_code" -eq 2
    RAYON_NUM_THREADS=$threads target/debug/qlrs reduce "$qlrs_temp/input.jsonl" --output "$qlrs_temp/bundle-$threads" > "$qlrs_temp/reduce-$threads.json"
    target/debug/qlrs bundle verify "$qlrs_temp/bundle-$threads" > "$qlrs_temp/bundle-$threads.json"
done
diff -u "$qlrs_temp/verify-1.json" "$qlrs_temp/verify-8.json"
diff -u "$qlrs_temp/reduce-1.json" "$qlrs_temp/reduce-8.json"
diff -r "$qlrs_temp/bundle-1" "$qlrs_temp/bundle-8"
echo 'one/eight-thread verification and bundle determinism: PASS'
