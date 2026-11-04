#!/usr/bin/env python3
"""Independently compare actual CLI verdicts to the protected TOML catalogue."""
import json,subprocess,tempfile,tomllib
from pathlib import Path
catalogue=tomllib.loads(Path('fixtures/defects/catalogue.toml').read_text())
subprocess.run(['cargo','build','-p','qlrs-cli','-p','qlrs-testkit','--examples','--locked'],check=True)
subprocess.run(['cargo','build','-p','qlrs-cli','--locked'],check=True)
with tempfile.TemporaryDirectory(prefix='qlrs-catalogue-') as temp:
 path=Path(temp)/'trace.jsonl'
 for defect in catalogue['defect']:
  subprocess.run(['target/debug/examples/generate','4',str(path),defect['id'],'2'],check=True)
  result=subprocess.run(['target/debug/qlrs','verify',str(path)],capture_output=True,text=True)
  report=json.loads(result.stdout)
  assert report['status']==defect['status'],f"{defect['id']}: {report}"
  if defect['invariant']:assert any(f['instance_key'] and f['instance_key']['invariant_id']==defect['invariant'] for f in report['findings'])
print('protected catalogue CLI matrix: PASS')
