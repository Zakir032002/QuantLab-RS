# Benchmark reproduction and scope

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
