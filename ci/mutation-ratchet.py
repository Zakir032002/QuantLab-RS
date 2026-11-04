#!/usr/bin/env python3
"""Require complete campaigns and nondecreasing floors for each audited scope."""
import argparse
import collections
import json
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('outcomes',type=Path)
parser.add_argument('enumeration',type=Path,nargs='?')
parser.add_argument('--legacy',action='store_true',help='explicitly validate an archived num/check-only campaign')
args=parser.parse_args()
result=json.loads(args.outcomes.read_text())
baseline=json.loads(Path('ci/mutants-baseline.txt').read_text())
enumeration=args.enumeration or args.outcomes.with_name('mutants.json')
assert result['end_time'] is not None,'mutation campaign did not complete'
assert result['outcomes'][0]['scenario']=='Baseline' and result['outcomes'][0]['summary']=='Success','unmutated tests failed'
assert result['total_mutants']==len(json.loads(enumeration.read_text())),'some mutations did not run'
assert result['timeout']==0,'timeouts require diagnosis; not silently counted as kills'
mutants=[item for item in result['outcomes'] if isinstance(item['scenario'],dict) and 'Mutant' in item['scenario']]
assert len(mutants)==result['total_mutants'],'missing mutant outcomes'
packages={item['scenario']['Mutant']['package'] for item in mutants}
assert {'qlrs-num','qlrs-check'}<=packages,'legacy package coverage missing'
scopes={
 'legacy':[item for item in mutants if item['scenario']['Mutant']['package'] in {'qlrs-num','qlrs-check'}],
 'reducer':[item for item in mutants if item['scenario']['Mutant']['package']=='qlrs-reduce'],
 'fingerprints':[item for item in mutants if item['scenario']['Mutant']['file']=='crates/qlrs-check/src/fingerprint.rs'],
}
for name,items in scopes.items():
 if args.legacy and name!='legacy':continue
 assert items,f'no mutations executed for required scope: {name}'
 counts=collections.Counter(item['summary'] for item in items)
 assert set(counts)<={'CaughtMutant','MissedMutant','Unviable'},f'unexpected outcomes in {name}: {counts}'
 caught=counts['CaughtMutant'];viable=caught+counts['MissedMutant']
 assert viable>0,f'zero viable mutations: {name}'
 floor=baseline if name=='legacy' else baseline.get('scopes',{}).get(name)
 assert floor and floor['viable']>0,f'no measured baseline recorded for {name}'
 assert caught*floor['viable']>=floor['caught']*viable,f'mutation score decreased in {name}: {caught}/{viable}'
 print(f'mutation ratchet {name}: PASS ({caught}/{viable}; {counts["Unviable"]} unviable)')
