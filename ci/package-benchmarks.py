#!/usr/bin/env python3
"""Retain raw Criterion samples and a public, credential-free benchmark record."""
import base64
import hashlib
import io
import json
import platform
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

output = Path(sys.argv[1])
output.mkdir(parents=True, exist_ok=True)
criterion = Path('target/criterion')
names = {'benchmark.json', 'estimates.json', 'sample.json', 'tukey.json'}
workloads = {'decimal_parse', 'mul_rescale_i128', 'mul_rescale_wide',
             'pipeline/check_100k', 'pipeline/parse_100k',
             'pipeline/replay_100k', 'reduce_100k'}
for workload in sorted(workloads):
    for name in sorted(names):
        source = criterion / workload / 'new' / name
        destination = output / 'criterion' / source.relative_to(criterion)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)


def command(args):
    return subprocess.check_output(args, text=True)


machine = platform.platform() + '\n'
if platform.system() == 'Darwin':
    machine += command(['sw_vers'])
    machine += command(['sysctl', 'machdep.cpu.brand_string', 'hw.ncpu', 'hw.memsize'])
else:
    machine += command(['lscpu'])
(output / 'machine.txt').write_text(machine)
(output / 'rustc.txt').write_text(command(['rustc', '-Vv']))
(output / 'git-commit.txt').write_text(command(['git', 'rev-parse', 'HEAD']))
sources = sorted(p for p in Path('crates').rglob('*')
                 if p.is_file() and p.suffix in {'.rs', '.toml'})
sources += [Path('Cargo.toml'), Path('Cargo.lock'), Path('rust-toolchain.toml')]
(output / 'source-sha256.txt').write_text(''.join(
    hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.as_posix() + '\n'
    for p in sources))

rows = []
for path in sorted((output / 'criterion').rglob('estimates.json')):
    sample = json.loads(path.with_name('sample.json').read_text())
    assert len(sample['times']) == 50, 'benchmark did not collect 50 samples'
    data = json.loads(path.read_text())['median']
    interval = data['confidence_interval']
    workload = path.parent.parent.relative_to(output / 'criterion').as_posix()
    rows.append(f"| {workload} | {data['point_estimate']:.3f} | "
                f"{interval['lower_bound']:.3f}–{interval['upper_bound']:.3f} |")
assert len(rows) == 7, 'one or more benchmark workloads did not execute'
(output / 'MEDIANS.md').write_text(
    '# Criterion medians\n\nNanoseconds per iteration; 50 samples.\n\n'
    '| Workload | Median ns | Median confidence interval ns |\n'
    '|---|---:|---:|\n' + '\n'.join(rows) + '\n')
(output / 'COMMANDS.md').write_text('''# Benchmark reproduction and scope

RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50

Rust 1.99.1, default portable CPU target, system allocator, fat LTO and overflow
checks enabled. Criterion default warm-up/measurement configuration; 50 samples.
The pipeline contains 100k events, 20% fills; replay is single-threaded and
parsing uses five Rayon workers. Buffer cloning is outside the timed parse.
Fixtures use seed 42, 20000 orders; the D03 reduction case injects at order 12000.
Actual fixture digests, source hashes, source commit, CPU/OS and compiler are
retained alongside all seven raw estimates/sample/benchmark/tukey JSON sets.
These are shared hosted-runner measurements, not dedicated-machine guarantees.
The exact workflow commands also build the existing generate/hash examples to
commit the actual inputs; those commands run after the timed benchmarks.
''')

# Public records contain only the requested numeric and provenance files.
# The full build console remains in the ordinary Actions artifact.
metadata = {'COMMANDS.md', 'MEDIANS.md', 'machine.txt', 'rustc.txt',
            'git-commit.txt', 'source-sha256.txt', 'input-digests.txt'}
files = sorted([output / name for name in metadata] + [
    output / 'criterion' / workload / 'new' / name
    for workload in workloads for name in names])
(output / 'MANIFEST.sha256').write_text(''.join(
    hashlib.sha256(p.read_bytes()).hexdigest() + '  '
    + p.relative_to(output).as_posix() + '\n' for p in files))
files.append(output / 'MANIFEST.sha256')
archive = io.BytesIO()
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
    for path in sorted(files):
        bundle.write(path, path.relative_to(output).as_posix())
payload = archive.getvalue()
(output / 'raw-benchmarks.zip').write_bytes(payload)
digest = hashlib.sha256(payload).hexdigest()
encoded = base64.b64encode(payload).decode()
chunks = [encoded[i:i + 3600] for i in range(0, len(encoded), 3600)]
assert len(chunks) <= 10, 'record exceeds the public annotation budget'
# Only the explicit benchmark files above enter the record: no environment,
# credentials, home directories, caches or arbitrary workspace contents.
# Public HTML truncates a single message at 4096 characters. Each bounded
# record is independently numbered and the complete ZIP has one commitment.
# The same ZIP remains available as an ordinary Actions artifact.
for index, chunk in enumerate(chunks, 1):
    print(f'::notice title=QLRS_RAW_BENCHMARK_ARCHIVE_V1_{index}_OF_{len(chunks)}::'
          f'{digest}:{index}:{len(chunks)}:{chunk}')
