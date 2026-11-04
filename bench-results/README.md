# Measured Part 1 results

Both platforms use Rust 1.99.1, the same committed Rust sources and seed-42
inputs, portable CPU targets, the system allocator and enabled overflow checks.
Each Criterion workload retains 50 samples and median confidence intervals.

| Platform | Machine | Raw results |
|---|---|---|
| Linux x86_64 | Shared AMD EPYC cloud instance | [Commands, samples and gates](2026-11-04-linux-x86_64/) |
| macOS arm64 | Hosted Apple M1 virtual runner, three CPUs | [Commands and samples](2026-11-04-macos-arm64/) |

These are observations on the recorded machines. The pipeline has 20% fill
events; replay is single-threaded and parsing uses five Rayon workers.
See docs/HANDOFF_PART_01.md for the measured results and unmet engineering goals.

The macOS numeric JSON files match the original artifact's per-file commitments.
PROVENANCE.json and TRANSFER.json record the original run and verified export.
The public subset retains CPU/OS/compiler fields and omits hostname/build-path
console details. MANIFEST.sha256 commits the archived raw files; COMMITTED.sha256
also covers the receipt and stored public ZIP. The original complete artifact
remains on the linked workflow run.
