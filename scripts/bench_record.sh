#!/usr/bin/env bash
set -euo pipefail
python3 - "$@" <<'PY'
import datetime, hashlib, json, os, pathlib, platform, subprocess, sys, time
out=pathlib.Path(sys.argv[1]);command=sys.argv[2:]
if not command:raise SystemExit('usage: bench_record.sh OUTPUT_DIRECTORY COMMAND [ARGS...]')
out.mkdir(parents=True,exist_ok=True)
meta={'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'command':command,'host':platform.node(),'os':platform.platform(),'cpu':next((x.split(':',1)[1].strip() for x in pathlib.Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),'not available') if platform.system()=='Linux' else subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),'cores':os.cpu_count(),'rustc_vv':subprocess.check_output(['rustc','-Vv'],text=True),'runner_image':{k:os.environ.get(k) for k in ('ImageOS','ImageVersion')},'source_sha256':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(pathlib.Path('crates').rglob('*.rs'))},'environment':{k:os.environ.get(k) for k in ('RAYON_NUM_THREADS','GUNGRAUN_ALLOW_ASLR','CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS','ASAN_OPTIONS')}}
meta['fuzz_sha256']={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(pathlib.Path('fuzz').glob('**/*')) if p.is_file() and ('target' not in p.parts) and ('corpus' not in p.parts) and ('artifacts' not in p.parts) and p.stat().st_size < 1024*1024}
(out/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
start=time.monotonic()
with (out/'stdout.txt').open('wb') as stdout,(out/'stderr.txt').open('wb') as stderr:
 result=subprocess.run(command,stdout=stdout,stderr=stderr)
meta.update(exit_code=result.returncode,elapsed_seconds=time.monotonic()-start)
(out/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n')
raise SystemExit(result.returncode)
PY
