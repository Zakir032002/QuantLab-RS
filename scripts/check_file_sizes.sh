#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
import pathlib, subprocess
allow=set(pathlib.Path('scripts/large_file_allowlist.txt').read_text().splitlines())
paths=subprocess.check_output(['git','ls-files','-z']).decode().split('\0')
bad=[p for p in paths if p and p not in allow and pathlib.Path(p).stat().st_size>5*1024*1024]
if bad:raise SystemExit('tracked files exceed 5 MiB: '+', '.join(bad))
print('tracked file-size gate: PASS')
PY
