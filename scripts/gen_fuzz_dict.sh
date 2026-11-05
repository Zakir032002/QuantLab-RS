#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
import json,pathlib,re
spec=pathlib.Path('docs/SPEC.md').read_text()
def block(name):return spec.split('<!-- '+name+' -->')[1].split('```text\n')[1].split('```')[0].split()
keys=block('qlrs-schema-keys')
rust=pathlib.Path('crates/qlrs-io/src/limits.rs').read_text().split('pub const SCHEMA_KEYS: &[&str] = &[')[1].split('];')[0]
assert keys==re.findall(r'"([^"]+)"',rust),'schema dictionary drift'
tokens=keys+block('qlrs-schema-values')+['{','}','[',']',':',',','"',r'\u0000',r'\ud800','\n','\r\n','true','false','null','-','.','e','E','+','9223372036854775807','170141183460469231731687303715884105727','340282366920938463463374607431768211455','1000000000000000000']
p=pathlib.Path('fuzz/dict');p.mkdir(parents=True,exist_ok=True)
def quote(token):
    return '"'+''.join('\\'+chr(b) if b in (34,92) else chr(b) if 32 <= b < 127 else f'\\x{b:02x}' for b in token.encode())+'"'
(p/'qlrs_trace_v0.dict').write_text('\n'.join(quote(token) for token in dict.fromkeys(tokens))+'\n')
(p/'decimal.dict').write_text('\n'.join(json.dumps(t) for t in list('0123456789-.eE+'))+'\n')
PY
