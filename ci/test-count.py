#!/usr/bin/env python3
"""Read the actual nextest list and enforce a monotonic minimum."""
import json,subprocess
from pathlib import Path
result=json.loads(subprocess.check_output(['cargo','nextest','list','--workspace','--locked','--message-format','json']))
minimum=int(Path('ci/test-count.txt').read_text());actual=result['test-count']
assert actual>=minimum, f'test count fell: {actual} < {minimum}'
print(f'test count ratchet: PASS ({actual} >= {minimum})')
