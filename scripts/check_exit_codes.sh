#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked -p qlrs-cli
python3 - <<'PY'
import json,pathlib,subprocess
trace='fixtures/clean/C01-worked-example.jsonl'
for args,code,error in [([],0,None),(['--max-events','4'],4,'U-LIMIT-EVENTS'),(['--max-interned','1'],4,'U-LIMIT-INTERNED'),(['--max-line-bytes','16'],1,'E-LIMIT-LINE-BYTES'),(['--max-input-bytes','1'],1,'E-LIMIT-INPUT-BYTES'),(['--memory-budget','0'],3,'I-BUDGET-MEMORY')]:
 r=subprocess.run(['target/debug/qlrs','verify',trace,*args],capture_output=True,text=True)
 assert r.returncode==code,(args,r.returncode,r.stderr)
 if error:
  report=json.loads(r.stdout);assert report['code']==error and error in r.stderr.splitlines()[0]
 print('PASS',code,error or 'CONSISTENT')
PY
