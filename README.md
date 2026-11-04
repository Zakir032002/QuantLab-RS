# QuantLab-RS

`qlrs` independently audits synthetic financial traces and reduces contradictions
to small, lifecycle-valid, replayable witnesses. Part 1 implements a cash account,
one currency, spot market orders, partial fills, and non-negative fees.

Money uses checked i128 fixed point with per-value scales and a cold 256-bit
arithmetic path. Replay, checking and reduction use per-kind columns and dense
IDs. JSONL stays at the boundary; observed lines are preserved byte-for-byte.

## Run

Rust **1.99.1**, rustfmt and clippy are pinned. No database, service or credentials
are needed for the synthetic workflow.

```sh
cargo build --release --locked -p qlrs-cli
./target/release/qlrs verify fixtures/clean/C01-worked-example.jsonl
```

The worked example computes `10000.00 - 20 * 100.00 - 10.00 = 7990.00`.
Verification emits deterministic JSON. Its exit status is:

| Outcome | Exit |
|---|---:|
| CONSISTENT | 0 |
| Malformed input or command operation failure | 1 |
| VIOLATION | 2 |
| INCONCLUSIVE | 3 |
| UNSUPPORTED | 4 |

`CONSISTENT` means no contradiction was found under the declared model; it is
not proof of an engine's correctness. Unsupported inputs suppress violations
unless `--partial` is requested. Partial checking reports supported evidence
before the first uncertain event.

## Generate and shrink a defect

```sh
qlrs_demo=$(mktemp -d)
cargo run -p qlrs-testkit --example generate --release --locked -- \
  2000 "$qlrs_demo/input.jsonl" D03 1400
./target/release/qlrs verify "$qlrs_demo/input.jsonl" # expected exit 2
./target/release/qlrs reduce "$qlrs_demo/input.jsonl" --output "$qlrs_demo/witness"
./target/release/qlrs bundle verify "$qlrs_demo/witness"
./target/release/qlrs verify "$qlrs_demo/witness/reduced_trace.jsonl" \
  --checkpoint "$qlrs_demo/witness/synthetic_checkpoint.json" # expected exit 2
```

Reduction preserves the exact target instance, order lifecycles and observed
bytes. Removed prefix history is replaced by explicitly **synthetic** state
computed by the independent ledger. The guarantee is **closure-1-minimal relative
to pinned units**, not a global minimum. This is a Mode T output witness, not a
reproduction from reduced engine inputs. Residual relations are heuristic hints,
not diagnoses.

Bundles contain the manifest, violation, reduced JSONL, optional synthetic
checkpoint, complete predicate log, reproduction instructions and a detached
manifest checksum. Verification checks every file and replays the target.
Integrity checks detect corruption; they do not authenticate the bundle author.

## Develop

```sh
./ci/check-all.sh
python3 ci/catalogue.py
cargo run -p qlrs-testkit --example scale_gate --release --locked
cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50
```

Install the pinned validation tools in [STACK.md](docs/STACK.md) before the full
check script. Linux instruction benches also need Valgrind and gungraun-runner.
Cloud shell activation:

```sh
export RUSTUP_HOME=/workspace/.qlrs-tools/rustup
export CARGO_HOME=/workspace/.qlrs-tools/cargo
export PATH="/workspace/.qlrs-tools/bin:$CARGO_HOME/bin:$PATH"
```

Read [the handoff](docs/HANDOFF_PART_01.md) for actual validation, performance
measurements and limitations; [SPEC.md](docs/SPEC.md) for the boundary and numeric
contract; and [ADRs](docs/adr/) for decisions and disclosed deviations. Raw Linux
benchmark results, confidence intervals and machine information are committed in
[Linux results](bench-results/2026-11-04-linux-x86_64/) and
[macOS arm64 results](bench-results/2026-11-04-macos-arm64/). Hosted Linux/macOS
quality, Miri and scale/instruction jobs passed for the recorded source: see the
[complete run](https://github.com/Zakir032002/QuantLab-RS/actions/runs/41836096568).

NautilusTrader/LEAN adapters, multi-currency, shorting/margin, other order types,
corporate actions, web UI and engine re-run minimization are outside Part 1.
