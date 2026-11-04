#!/usr/bin/env python3
"""Compare measured Ir counts to the pinned Linux numeric baseline."""
import json,re,sys
from pathlib import Path
baseline=json.loads(Path('ci/instruction-baseline.json').read_text())
current={};name=None
for line in Path(sys.argv[1]).read_text().splitlines():
 match=re.match(r'instructions::numeric::(\w+)',line)
 if match:name=match[1]
 match=re.match(r'\s*Instructions:\s*([\d,]+)\|',line)
 if match and name:current[name]=int(match[1].replace(',',''))
for name,expected in baseline['instructions'].items():
 assert name in current, f'benchmark did not execute: {name}'
 assert current[name]*100<=expected*105, f'instruction regression {name}: {current[name]} > {expected} + 5%'
print('instruction count ratchet: PASS',current)
