#!/usr/bin/env python3
"""Require a complete campaign, successful baseline, and non-decreasing score."""
import json,sys
from pathlib import Path
path=Path(sys.argv[1]);result=json.loads(path.read_text());baseline=json.loads(Path('ci/mutants-baseline.txt').read_text())
assert result['end_time'] is not None,'mutation campaign did not complete'
assert result['outcomes'][0]['scenario']=='Baseline' and result['outcomes'][0]['summary']=='Success','unmutated tests failed'
enumeration=Path(sys.argv[2]) if len(sys.argv)>2 else path.with_name('mutants.json')
assert result['total_mutants']==len(json.loads(enumeration.read_text())),'some mutations did not run'
assert result['timeout']==0,'timeouts require diagnosis; not silently counted as kills'
denominator=result['caught']+result['missed']
assert denominator>0,'zero viable mutations'
assert result['caught']*baseline['viable']>=baseline['caught']*denominator,'mutation score decreased'
print(f"mutation ratchet: PASS ({result['caught']}/{denominator}; {result['unviable']} unviable)")
