#!/usr/bin/env python3
"""Derive an auditable report from a complete campaign and exact-site rechecks.

Raw campaigns are never rewritten. Only previously missed/timed-out mutations
may be replaced; the entire mutation description must match byte-for-byte.
"""
import argparse,collections,hashlib,json
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('campaign',type=Path)
p.add_argument('enumeration',type=Path)
p.add_argument('output',type=Path)
p.add_argument('rechecks',type=Path,nargs='+')
a=p.parse_args()

def read(path):
    result=json.loads(path.read_text())
    assert result['end_time'] is not None,f'incomplete run: {path}'
    assert result['outcomes'][0]['scenario']=='Baseline' and result['outcomes'][0]['summary']=='Success',f'failed baseline: {path}'
    return result

def key(mutant):return json.dumps(mutant,sort_keys=True,separators=(',',':'))

base=read(a.campaign)
assert base['total_mutants']==len(json.loads(a.enumeration.read_text())),'incomplete enumeration'
indices={key(row['scenario']['Mutant']):index for index,row in enumerate(base['outcomes']) if isinstance(row['scenario'],dict)}
assert len(indices)==base['total_mutants'],'duplicate/missing outcomes'
provenance=[{'path':str(a.campaign),'sha256':hashlib.sha256(a.campaign.read_bytes()).hexdigest(),'kind':'complete campaign'}]
replaced=[]
for path in a.rechecks:
    run=read(path)
    assert run['timeout']==0,f'unresolved timeout: {path}'
    checked=[row for row in run['outcomes'] if isinstance(row['scenario'],dict)]
    assert len(checked)==run['total_mutants'],'incomplete recheck'
    for row in checked:
        identity=key(row['scenario']['Mutant'])
        assert identity in indices,'recheck changes production mutation/source'
        index=indices[identity];old=base['outcomes'][index]
        assert old['summary'] in {'MissedMutant','Timeout'},'recheck is not resolving a survivor/timeout'
        assert row['summary'] in {'CaughtMutant','MissedMutant'},'recheck did not execute tests'
        assert any('--workspace' in phase['argv'] for phase in row['phase_results'] if phase['phase']=='Test'),'recheck narrowed test scope'
        replaced.append({'name':row['scenario']['Mutant']['name'],'was':old['summary'],'now':row['summary'],'run':str(path)})
        base['outcomes'][index]=row
    provenance.append({'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'kind':'focused exact-site recheck'})
    base['end_time']=run['end_time']
counts=collections.Counter(row['summary'] for row in base['outcomes'] if isinstance(row['scenario'],dict))
assert counts['Timeout']==0,'timeouts remain unresolved'
for field,summary in [('caught','CaughtMutant'),('missed','MissedMutant'),('unviable','Unviable'),('timeout','Timeout')]:base[field]=counts[summary]
assert sum(counts.values())==base['total_mutants'],'coverage changed'
base['report_kind']='derived complete coverage with focused rechecks; not a new raw campaign'
base['raw_campaigns']=provenance
base['replaced_outcomes']=replaced
a.output.write_text(json.dumps(base,indent=2)+'\n')
print(f'validated complete coverage: {base["total_mutants"]} mutations, {len(replaced)} exact-site rechecks')
