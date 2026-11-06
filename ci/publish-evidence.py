#!/usr/bin/env python3
"""Publish bounded, explicitly selected measurement logs with a SHA256 commitment."""
import base64
import hashlib
import io
import sys
import zipfile
from pathlib import Path

root = Path(sys.argv[1])
files = sorted(p for p in root.rglob('*') if p.is_file()
               and p.name in {'metadata.json', 'stdout.txt', 'stderr.txt', 'comparison.json'})
assert files, 'no measurement logs to publish'
assert all(p.stat().st_size < 1024 * 1024 for p in files), 'trim oversized logs explicitly'
manifest = ''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  '
                   + p.relative_to(root).as_posix() + '\n' for p in files)
archive = io.BytesIO()
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
    for path in files:
        bundle.write(path, path.relative_to(root).as_posix())
    bundle.writestr('MANIFEST.sha256', manifest)
payload = archive.getvalue()
digest = hashlib.sha256(payload).hexdigest()
encoded = base64.b64encode(payload).decode()
chunks = [encoded[i:i + 3000] for i in range(0, len(encoded), 3000)]
assert len(chunks) <= 50, 'measurement record exceeds public annotation budget'
for index, chunk in enumerate(chunks, 1):
    print(f'::notice title=QLRS measurement {index} of {len(chunks)}::'
          f'{digest}:{index}:{len(chunks)}:{chunk}')
