#!/usr/bin/env python3
"""Reject reverse runtime edges and boundary-only dependency leaks."""
import json,subprocess,sys
m=json.loads(subprocess.check_output(['cargo','metadata','--format-version=1','--no-deps','--locked']))
allowed={'num':set(),'schema':{'num'},'io':{'schema','num'},'ledger':{'schema','num'},'check':{'schema','num','ledger'},'reduce':{'schema','num','ledger','check'},'bundle':{'schema','check','reduce','io'},'cli':{'num','schema','io','ledger','check','reduce','bundle'},'testkit':{'num','schema','io','ledger','check','reduce','bundle'},'bench':{'num','schema','io','ledger','check','reduce','testkit'}}
errors=[]
for p in m['packages']:
 name=p['name'];short=name.removeprefix('qlrs-')
 for dep in p['dependencies']:
  n=dep['name'];kind=dep['kind']
  if n in ('serde_json','sonic-rs') and short!='io':errors.append(f'{name} -> {n}: JSON is I/O-only')
  if n in ('clap','tracing-subscriber','mimalloc') and short!='cli':errors.append(f'{name} -> {n}: CLI-only')
  if n.startswith('qlrs-') and kind is None and n.removeprefix('qlrs-') not in allowed[short]:errors.append(f'{name} -> {n}: forbidden runtime edge')
if errors:sys.exit('\n'.join(errors))
print('dependency direction: PASS')
