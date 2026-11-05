# Fuzzing

Use the pinned toolchain in rust-toolchain.toml and cargo-fuzz 0.13.2. Oracles
live in qlrs-testkit and normal workspace tests replay committed corpora on
stable Rust. No product check is disabled under cfg(fuzzing).

From the repository root:

```sh
cargo +nightly-2026-11-04 fuzz run --fuzz-dir fuzz --release -s none num_ops -- -max_total_time=60 -rss_limit_mb=2048 -timeout=10 -print_final_stats=1
cargo +nightly-2026-11-04 fuzz cmin --fuzz-dir fuzz --release -s none num_ops -- -rss_limit_mb=2048 -timeout=10 -print_final_stats=1
```

Parser ASan smoke uses `-s address`; the cloud sandbox also needs
`ASAN_OPTIONS=detect_leaks=0` because its leak scanner cannot use ptrace.
Dictionary paths are relative to the command's working directory.

io_chunk_diff's first three bytes encode a little-endian chunk size minus one
and worker count minus one modulo eight; remaining bytes are the trace. The
one-byte sequential chunk path and selected rayon pool must yield identical
columns, interner IDs, source references and first errors.

Run scripts/gen_fuzz_dict.sh after changing the schema vocabulary. Minimize
before corpus commits. Keep each corpus below 2,000 files / 20 MiB and each
tracked file below 5 MiB. A crash requires a minimized regression commit before
a separate fix; do not overwrite a crash artifact or count setup failures as
fuzz time. See ADR-0018 and PART_03_INSTRUCTIONS for campaign accounting.
