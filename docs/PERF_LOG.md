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
