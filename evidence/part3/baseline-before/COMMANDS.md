# Before baseline

Activate /tmp/qlrs-env. scripts/bench_record.sh stores exact argv, source hashes,
compiler/CPU/host, exit status and raw stdout/stderr for every command.
The GNU time executable is the verified Debian package under the workspace.

- Generate 100k/1M with the existing generate example, seed 42.
- Verify each with GNU time -v; hyperfine 2.0.0 runs 1M with two warmups and ten runs.
- GUNGRAUN_ALLOW_ASLR=yes cargo bench -p qlrs-bench --bench instructions --locked.
- RAYON_NUM_THREADS=5 cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50.

No source optimization was made during measurement. Release profile remains
fat LTO, one codegen unit, checked arithmetic and panic=abort. Criterion raw
JSON and input commitments are included. Machine is cloud Linux x86_64; Apple
Silicon is pending. No generated trace is committed.
