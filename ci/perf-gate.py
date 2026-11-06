#!/usr/bin/env python3
"""Compare actual same-runner Callgrind instruction counts, with integer gating."""
import json,pathlib,re,sys

def counts(path):
    result={};current=None
    for line in pathlib.Path(path).read_text().splitlines():
        if line.startswith('instructions::'):current=line.strip()
        match=re.match(r'\s*Instructions:\s*(\d+)\|',line)
        if match:
            assert current and current not in result,'missing or duplicate benchmark'
            result[current]=int(match[1])
    assert result and all(result.values()),'missing or zero instruction counts'
    return result
base,head=map(counts,sys.argv[1:3]);assert base.keys()==head.keys(),'benchmark sets differ'
rows=[{'benchmark':name,'base':value,'head':head[name],'pass':head[name]*100<=value*103} for name,value in base.items()]
print(json.dumps({'threshold_percent':3,'benchmarks':rows,'pass':all(row['pass'] for row in rows)},indent=2))
raise SystemExit(0 if all(row['pass'] for row in rows) else 1)
