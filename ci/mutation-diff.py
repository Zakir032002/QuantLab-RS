#!/usr/bin/env python3
"""Validate filtered PR mutations without pretending they are a full campaign."""
import hashlib,json,sys
from pathlib import Path

def identity(mutant):
    # Exact source site: moving/changing a site requires fresh triage rather
    # than silently inheriting an acceptance from another expression.
    key=[mutant['file'],(mutant.get('function') or {}).get('function_name'),mutant['genre'],mutant['replacement'],mutant['span']]
    return hashlib.sha256(json.dumps(key,sort_keys=True,separators=(',',':')).encode()).hexdigest()


if __name__=='__main__':
    result=json.loads(Path(sys.argv[1]).read_text())
    accepted={entry['id'] for entry in json.loads(Path('ci/mutants-accepted.json').read_text())}
    assert result['end_time'] is not None and result['timeout']==0,'incomplete campaign or timeout'
    assert result['outcomes'][0]['scenario']=='Baseline' and result['outcomes'][0]['summary']=='Success','baseline failed'
    mutants=[entry for entry in result['outcomes'] if isinstance(entry['scenario'],dict)]
    assert len(mutants)==result['total_mutants'],'missing filtered outcomes'
    missed=[entry for entry in mutants if entry['summary']=='MissedMutant' and identity(entry['scenario']['Mutant']) not in accepted]
    assert not missed,'new unreviewed survivors: '+', '.join(entry['scenario']['Mutant']['name'] for entry in missed)
    print(f'filtered PR mutation check: PASS ({len(mutants)} outcomes; full separate floors remain nightly)')
