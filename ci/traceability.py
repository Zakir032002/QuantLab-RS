#!/usr/bin/env python3
"""Ensure every formal requirement is mapped to existing test functions."""
from pathlib import Path
import re,sys
spec=set(re.findall(r'REQ-[A-Z]+-\d+',Path('docs/SPEC.md').read_text()))
trace=Path('docs/TRACE.md').read_text()
tests=set()
for path in Path('crates').rglob('*.rs'):tests.update(re.findall(r'\bfn\s+(req_[a-z0-9_]+)\s*\(',path.read_text()))
errors=[]
for req in spec:
 rows=[row for row in trace.splitlines() if req in row]
 if not rows or not any(test in row for row in rows for test in tests):errors.append(req)
if errors:sys.exit('requirements missing tests: '+', '.join(sorted(errors)))
print(f'traceability: PASS ({len(spec)} requirements)')
