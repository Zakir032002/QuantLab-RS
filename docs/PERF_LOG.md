# Part 3 performance log

CP3.1 before: unchanged product source, c3ffd8b. Commands, compiler, host,
source hashes, raw stdout/stderr, GNU-time peaks, hyperfine JSON and 50-sample
Criterion data are in evidence/part3/baseline-before. Input SHA256/lengths are
recorded separately. Apple Silicon baseline: pending Zakir or a real ARM CI run.
No Intel sample is substituted for an ARM requirement.

R4 reopened: unchanged checker source passed the unchanged 12× / 0.5-second
bounds on this host. The old miss remains recorded. This wall-time change is not
attributed to a code change or an instruction-count improvement.

Future changes are retained only with measured instruction or memory improvements
and unchanged protected outputs. Streaming generation and source non-retention
are the first memory targets; parser/allocator alternatives remain unadopted.

CP3.2: new 10M streaming generator peaks at 2488 KiB, below the 64 MiB
goal; the old 1M Vec generator peaked at 175248 KiB. Different preset seeds
are labeled in raw metadata; no wall-time speedup is inferred from that comparison.
See evidence/part3/gen/s10m and baseline-before/generate-s1m. Repeated 1M
bytes/metadata are identical. Cross-OS confirmation is pending.

Part 3 streaming and storage: the original 1M input's retained-source path peaked
at 332436 KiB; streamed compact state peaks at 142824 KiB. The 10M run initially
used 1561384 KiB and missed its RSS gate; compact units, five lifecycle counters
and u32 trade keys lower it to 1388056 KiB and pass that bound. These are Linux
cloud development measurements with recorded worktree hashes, not Apple Silicon
results or a wall-time speedup claim. Raw paths:
`evidence/part3/baseline-before/verify-s1m`, `streaming/verify-s10m`, and
`hot-path/compact-state`. Its comparison.json passes all seven 3% instruction
checks against hot-path/before, measured on the same host.

The fixed interner is required for deterministic construction; its instruction
change alone is below the noise floor and is not a speed claim. Replacing the
builder's hash map from dense interned IDs to order IDs with a vector reduces
parse_100k from 293475388 to 262736014 instructions on the same host (about 10%).
Raw fixed-interner and dense-orders outputs and their source hashes are under
`evidence/part3/hot-path`; dense-orders/comparison.json compares all seven cases.
