# QuantLab-RS

An independent exact ledger and dependency-aware counterexample reducer for synthetic financial traces.

Use the existing isolated checkout; do not create a worktree unless requested.
For current Part 2 work, read `docs/PART_02_REBASE.md` first; it overrides stale
Part 1/Part 2 assumptions. Part 2 benchmark outputs use `bench/results/CP2.x/`.
Part 2 commits use `CP2.x: <objective>` and tags `part2-cp2.x` only when passed.

Read `docs/BLOCKERS.md`, `docs/HANDOFF_CLAIM_BOUNDARIES.md` and
`docs/PART_01_INSTRUCTIONS.md` before resuming. The corrections handoff records
the user's later requirements and current evidence; the original Part 1 handoff
is a historical baseline.

## Commands

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --doc --workspace --locked
cargo build -p qlrs-num --example numeric_probe --locked
python3 ci/numeric-oracle.py
```

Run ./ci/check-all.sh for the complete routine suite, python3 ci/catalogue.py
for the protected matrix, and the scale_gate/rich_gate examples for million-event evidence.
Miri and mutation coverage/ratchets are documented in the handoff and CI.

## Mandatory rules (verbatim from Part 1 section 4)

1. **No unverified versions.** Before adding any dependency, check crates.io and docs.rs for its current version and maintenance (a release within roughly 12 months, or explicit maintainer statement). Record version, date and URL in `docs/STACK.md`. Use `--locked` everywhere.
2. **No benchmark claims without running.** Every number in any document must come from a committed raw output file under `bench-results/<date>-<machine>/`, together with `rustc -Vv`, CPU model, OS version and the command used. Without a file, write "not measured".
3. **ADRs.** Record each non-trivial decision in `docs/adr/NNNN-title.md` (context, options, decision, consequences). ADR-0001 (numeric representation), ADR-0002 (event layout) and ADR-0003 (reduction predicate) are mandatory in Part 1.
4. **Never weaken tests or tolerances to pass.** Do not edit golden files, expected statuses, the defect catalogue, rounding policy, or tolerance settings to make a failing test pass. If the spec looks wrong, stop and report.
5. **Stop and report on blockers.** If a blocker lasts more than two attempts, write `docs/BLOCKERS.md` (symptom, what was tried, hypothesis) and stop that checkpoint.
6. **No floats in money paths.** `f32` and `f64` are forbidden in `qlrs-num`, `qlrs-schema`, `qlrs-ledger`, `qlrs-check` and `qlrs-reduce`. Enforce with clippy `float_arithmetic` (deny) in those crates and a CI grep for `f32|f64` in their `src/`.
7. **No fabricated results.** Do not claim a test passes, a bug is found, or a reduction happened unless a command output shows it.
8. **No `unsafe`** in Part 1 (`#ǃ[forbid(unsafe_code)]` in every crate). Any exception needs an ADR plus Miri coverage.
9. **Determinism.** The same input gives byte-identical output regardless of thread count, OS or run.


## Protected paths

`fixtures/golden/**`, `fixtures/defects/catalogue.toml`, `docs/SPEC.md`
requirement IDs, and `deny.toml` require an ADR and human approval before changes.
Never create a passing placeholder for an unfinished check.

## Checkpoint completion

Run every command and satisfy every acceptance criterion in section 12 of
`docs/PART_01_INSTRUCTIONS.md` before claiming a checkpoint. Never infer hosted macOS success from local Linux results.
Commit format: `cp1.N: <summary>`; tag `cp1.N` only for a completed checkpoint.
