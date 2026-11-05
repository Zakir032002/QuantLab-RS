# ADR-0018: pinned libFuzzer and shared replay oracles

Status: F01/F02/F05/F06 implemented; remaining targets follow their product paths.

The separate `fuzz/` workspace pins cargo-fuzz 0.13.2, libfuzzer-sys 0.4.13,
arbitrary 1.5.0 and nightly-2026-11-04. Official release/version evidence is in
STACK and evidence/part3/cp3.0. It does not change the root Rust 1.99.1 toolchain.
Targets only call `qlrs_testkit::oracles`; stable corpus replay calls the same
functions. There is no fuzz-only bypass in product code.

The arithmetic oracle uses independent BigInt operations and the existing SPEC
error precedence. Fixed has no arbitrary-division method: the division case is
its power-of-ten rescale; negation is zero minus the operand. Adding an unused
financial API solely for a target would add risk without testing existing code.

Always pass `-rss_limit_mb=2048 -timeout=10 -print_final_stats=1`. Parser smoke
runs use ASan; numeric targets can use no sanitizer. The cloud sandbox prevents
LeakSanitizer's ptrace-based leak scan, so local smoke sets
`ASAN_OPTIONS=detect_leaks=0`; ASan remains enabled. This is recorded, not a
claim of leak coverage. A dictionary setup failure is retained separately from
successful campaigns and is not counted as fuzz time.

The pinned tool supports cargo-fuzz `--jobs` and libFuzzer `-jobs/-workers`.
Long runs will record each worker's actual user/system CPU time; wall time alone
is insufficient. Only a final-stats log with metadata counts toward Gate S.

Dictionaries derive keys/enums from SPEC and verify the Rust key list.
libFuzzer dictionaries use hexadecimal control-byte escapes, not JSON's newline
escape. Corpora are minimized with `cargo fuzz cmin` before committing; cap each
target at 2,000 files and 20 MiB. Production crashes follow the separate
regression-then-fix commits required by R10.
