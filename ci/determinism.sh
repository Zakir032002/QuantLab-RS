#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
qlrs_temp=$(mktemp -d)
trap 'rm -rf "$qlrs_temp"' EXIT
cargo build -p qlrs-cli --locked
cargo build -p qlrs-testkit --example generate --example generate_rich --locked
qlrs_compare() {
    local qlrs_case=$1
    local qlrs_input=$2
    shift 2
    for qlrs_threads in 1 2 8; do
        set +e
        RAYON_NUM_THREADS=$qlrs_threads target/debug/qlrs verify "$qlrs_input" > "$qlrs_temp/$qlrs_case-verify-$qlrs_threads.json"
        qlrs_code=$?
        set -e
        test "$qlrs_code" -eq 2
        RAYON_NUM_THREADS=$qlrs_threads target/debug/qlrs reduce "$qlrs_input" --output "$qlrs_temp/$qlrs_case-bundle-$qlrs_threads" "$@" > "$qlrs_temp/$qlrs_case-reduce-$qlrs_threads.json"
        target/debug/qlrs bundle verify "$qlrs_temp/$qlrs_case-bundle-$qlrs_threads" --recheck-certificate > "$qlrs_temp/$qlrs_case-checked-$qlrs_threads.json"
    done
    diff -u "$qlrs_temp/$qlrs_case-verify-1.json" "$qlrs_temp/$qlrs_case-verify-2.json"
    diff -u "$qlrs_temp/$qlrs_case-reduce-1.json" "$qlrs_temp/$qlrs_case-reduce-2.json"
    diff -r "$qlrs_temp/$qlrs_case-bundle-1" "$qlrs_temp/$qlrs_case-bundle-2"
    diff -u "$qlrs_temp/$qlrs_case-verify-1.json" "$qlrs_temp/$qlrs_case-verify-8.json"
    diff -u "$qlrs_temp/$qlrs_case-reduce-1.json" "$qlrs_temp/$qlrs_case-reduce-8.json"
    diff -r "$qlrs_temp/$qlrs_case-bundle-1" "$qlrs_temp/$qlrs_case-bundle-8"
}
target/debug/examples/generate 2000 "$qlrs_temp/classic.jsonl" D03 1400
qlrs_compare classic "$qlrs_temp/classic.jsonl"
for qlrs_defect in D01 D02 D03 D04 D05 D06 D07 D08 D09 D10 D11; do
    target/debug/examples/generate_rich 2000 "$qlrs_temp/rich.jsonl" long-lived "$qlrs_defect" 0
    qlrs_compare "$qlrs_defect" "$qlrs_temp/rich.jsonl"
done
qlrs_compare limited "$qlrs_temp/classic.jsonl" --max-calls 1
echo 'one/two/eight-thread reports and complete bundles: classic, all D01-D11 rich cases, and call-limited output PASS'
