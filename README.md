# QuantLab-RS

`qlrs` independently audits synthetic financial traces and reduces contradictions
to small, lifecycle-valid, replayable witnesses. It supports a cash account,
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
./target/release/qlrs bundle verify "$qlrs_demo/witness" --recheck-certificate
./target/release/qlrs verify "$qlrs_demo/witness/reduced_trace.jsonl" \
  --checkpoint "$qlrs_demo/witness/synthetic_checkpoint.json" # expected exit 2
```

Reduction preserves the exact target instance and observed bytes. It cuts at the
preceding claim and carries expected cash, inventory and order status/filled
quantity in an explicitly **synthetic** checkpoint. Opening claims supply the
observed baseline; I1 and I2 check movement between consecutive matching claims.
Full-trace verification requires later cash and same-instrument position claims
for each fill. Missing evidence produces `INCONCLUSIVE` when no violation is
proven; a proven `VIOLATION` takes precedence and retains the missing-evidence
findings. Witnesses need only their target scope and necessary model support.

The default SBR search uses exact algebraic seeds, dependency closures, checked
prefix queries and full-checker admission. Select the comparison algorithm with
`--algorithm ddmin-closures`. The default search budgets are 30 seconds and 10,000 candidate calls. Override
them with `--time-limit-ms 2000 --max-calls 1024`. Exhaustion returns the best
verified witness with `closure_1_minimal: false` and a `stop_reason`. Completed
searches certify **closure-1-minimality relative to boundary pins and closures**.
The certificate reports candidate tests; zero is explicitly vacuous. Limits are
cooperative: input validation and a replay already in progress finish safely.
This is a Mode T output witness; residual relations are heuristic hints.

Bundles contain the manifest, violation, reduced JSONL, optional synthetic
checkpoint, witness roles, an offline closure certificate, compact RLE logs,
reproduction instructions and a detached
manifest checksum. Verification checks every file and replays the target.
Integrity checks detect corruption; they do not authenticate the bundle author.

## Develop

```sh
./ci/check-all.sh
python3 ci/catalogue.py
cargo run -p qlrs-testkit --example part2_gate --release --locked
cargo run -p qlrs-testkit --example part2_regressions --release --locked
cargo run -p qlrs-testkit --example scale_gate --release --locked
cargo run -p qlrs-testkit --example rich_gate --release --locked -- 250000 128 2000
cargo bench -p qlrs-bench --bench throughput --locked -- --sample-size 50
```

The Part 2 gate runs D01–D11 across S1–S9, including staggered claims, at
1k/10k/100k event budgets and seeds 1/2/3. It checks exact keys, scope, minimal
pins, real removals, certificates, exhaustive small-window quality and identical
bundles with 1/2/8 workers. Required sparse windows may exceed 32 events; bounds
are per case. See [GENERATOR.md](docs/GENERATOR.md) and the committed
[measurement report](docs/BENCH_PART_02.md), including disclosed misses.

The historical Part 1 extended gate also remains active across interleaved, long-lived and sparse
multi-instrument traces with carried-forward errors. Each case must keep at
most 128 events, finish within 2 seconds, preserve the target and observed bytes,
and replay from its checkpoint. Generate one such trace with:

```sh
cargo run -p qlrs-testkit --example generate_rich --release --locked -- \
  2000 /tmp/qlrs-rich.jsonl long-lived D11 0
```

Install the pinned validation tools in [STACK.md](docs/STACK.md) before the full
check script. Linux instruction benches also need Valgrind and gungraun-runner.
Cloud shell activation:

```sh
export RUSTUP_HOME=/workspace/.qlrs-tools/rustup
export CARGO_HOME=/workspace/.qlrs-tools/cargo
export PATH="/workspace/.qlrs-tools/bin:$CARGO_HOME/bin:$PATH"
```

Read [the Part 2 handoff](docs/HANDOFF_PART_02.md) for current validation,
mutation coverage, measurements and the remaining R4 acceptance miss;
[SPEC.md](docs/SPEC.md) for the contract; and [ADRs](docs/adr/) for decisions.
The [claim-boundary corrections](docs/HANDOFF_CLAIM_BOUNDARIES.md) and
[original Part 1 handoff](docs/HANDOFF_PART_01.md) retain earlier baselines.

NautilusTrader/LEAN adapters, multi-currency, shorting/margin, other order types,
corporate actions, web UI and engine re-run minimization are outside Part 1.
