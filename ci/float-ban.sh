#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if rg -n '\b(f32|f64)\b' crates/qlrs-{num,schema,ledger,check,reduce}/src; then
    echo 'floating-point type in a money crate' >&2
    exit 1
fi
if rg -n '\bunsafe\s+(impl|fn|trait|extern)|\bunsafe\s*\{' crates/*/src; then
    echo 'unsafe implementation code' >&2
    exit 1
fi
echo 'money type and safety ban: PASS'
