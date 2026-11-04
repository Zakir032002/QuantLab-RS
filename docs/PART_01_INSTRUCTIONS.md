# QuantLab-RS: Final-Project Verdict, Roadmap and Part 1 Agent Instructions (November 2026)

**Yes, this is the final project, and the flagship (a dependency-aware reducer for financial traces) still holds up.** Two of the competitor claims need correcting. **dev-belly/LedgerX** is real. It is a small Python ledger with an append-only journal of fills that it replays, but it reads only its own JSONL events, has no adapter for any external engine, and does no reduction. **FanatiSam/backtest-evidence** is real too, but it does **not** recompute backtest ledgers. It checks daily PnL, timestamp causality, out-of-sample dates and SHA-256 file manifests, and its own README says it does not "validate fills". My searches turned up no tool that imports traces from a real engine, recomputes cash and positions independently, and then shrinks the inconsistency to a small replayable counterexample. That combination is what QuantLab-RS does.

## TL;DR

- **Final project: yes.** Both competitor claims are weaker than first described. LedgerX is a self-contained Python journal with no external-engine import and no reducer. backtest-evidence does no ledger recomputation at all. The flagship, a dependency-aware financial trace reducer, has no direct competitor that I found. Describe it as "no existing tool we found combines X+Y+Z", never as "first".
- **Facts that change the design.** NautilusTrader's latest release is **2.0.0rc8** (released 28 October 2026 UTC, marked pre-release). It is not 2.0.0 final. It also adds commissions with fractional digits beyond the currency's precision, so **scale must be stored per value, not per currency**. LEAN issue #9819, the liquidating-distribution and delisting double count, is real (opened 23 September 2026), and fix PRs #9865 and #9878 are in flight. Rust stable is 1.99.1. Rust 1.100.0 is due 12 November 2026.
- **Part 1** is a thin slice that is still fast where it counts. It covers a single-currency cash account and market orders with fees, checked against three invariants: cash per fill, position equals the sum of fills, and lifecycle legality. It adds an i128 fixed-point core with a cold 256-bit slow path, a columnar replay core with JSONL only at the boundary, two-level fingerprints, a lifecycle-closure reducer with labelled synthetic checkpoints, bundle v0, planted-defect corpora and measured benchmarks. It is split into nine checkpoints (CP1.0 to CP1.8) and ends at Gate A-synthetic plus early Gate B-synthetic.

---

## Key Findings

| Question | Verified answer | Confidence |
|---|---|---|
| "LedgerX" repo | `github.com/dev-belly/LedgerX`. Python, stdlib plus `Decimal`, 12 commits, 1 star, 0 forks, no licence file visible on the repo page. | High (repo fetched) |
| LedgerX scope | "An append-only execution ledger for research fills". Long-only USD, 1e-8 USD quantum, SHA-256 hash chain, `verify` replays the journal. Reads its own event schema and sits "after an execution simulator such as TradeForge". No reduction. | High |
| "backtest-evidence" repo | `github.com/FanatiSam/backtest-evidence`. MIT, Python 3.10+, v0.1.0, 1 commit, 0 stars. | High (repo fetched) |
| backtest-evidence scope | PnL missingness and drawdown, `available_at <= decision_at <= executed_at`, OOS-window audit, SHA-256 manifest and verify. Says explicitly that it does not "validate fills". No ledger, no engine import, no reduction. | High |
| Existing financial trace reducer | None found. Generic reducers do exist (ddmin, HDD, Perses, ProbDD, WDD, GReduce, J-Reduce). | Medium (absence of evidence) |
| LEAN double count | Issue #9819, "Liquidating distribution plus delisting liquidation double-counts the position in Raw-mode backtests". Opened 23 September 2026 by AlexCatarino. PR #9865 opened, then superseded by #9878 (Martin-Molinero, 7 October 2026). Merge state not confirmed. | High for the issue, low for the merge state |
| NautilusTrader around 28 October 2026 | **2.0.0rc8**, "Released on 28th October 2026 (UTC)", tagged pre-release. 1.231.0 (2 August 2026) is the last 1.x stable release. | High |
| `CanonicalBacktestResult` | Present in nautilus-backtest 0.63.0 (2 September 2026). Schema `nautilus-backtest-result/v1`, BLAKE3 digest, `first_divergence` returning an RFC 6901 pointer. | High (source read) |
| Nautilus precision | High-precision: i128 (Price/Money) and u128 (Quantity), 16 decimals. Standard: 64-bit, 9 decimals. Python wheels are high-precision. Rust crates default to standard unless the `high-precision` feature is on. | High |
| Rust | 1.99.0 (1 October 2026), 1.99.1 (15 October 2026), edition 2024. 1.100.0 is on beta and due 12 November 2026, bringing opt-in Cargo dependency cooldowns. | High |

---

## PART A — Project intro for Zakir (plain language)

**What it is.** QuantLab-RS (`qlrs`) is an auditor for backtests. A backtester such as NautilusTrader reports what happened: orders, fills, fees, cash balances and positions. `qlrs` reads that record, works out independently what the cash and positions *should* be using exact integer arithmetic, and tells you whether the engine's numbers contradict themselves. When they do, it shrinks the evidence from perhaps a million events to the handful that still prove the same contradiction, then packages them as a replayable regression fixture.

**Who it is for.** Two groups. Quant developers who need to trust their backtest P&L, and engine maintainers (Nautilus, later LEAN) who want small, valid, reproducible bug reports in place of "my numbers look wrong".

**Worked example.** The account starts with ₹10,000.00 and buys 20 shares at ₹100.00 with a ₹10.00 fee. Expected cash is 10,000 − 20×100 − 10 = ₹7,990.00. The engine's next account snapshot says ₹8,090.00, so the residual is +₹100.00. `qlrs` reports `VIOLATION` of invariant I1 (cash conservation per fill) at that snapshot. The residual is exactly one share's notional, so the class fingerprint records "residual = +1 × last_px" as a clue (it is not a diagnosis). If this happened at event 731,402 of a million-event trace, the reducer removes every order lifecycle that is not needed. It keeps the two account snapshots around the fill verbatim and replaces the deleted history with a checkpoint explicitly labelled `SYNTHETIC`. The result is a trace of about five events that still shows exactly the same +₹100.00 residual at the same snapshot.

**Two modes.** Mode T (trace) checks recorded outputs only and is built first. Mode E (engine re-run) comes much later: it re-runs the engine on reduced inputs. A small witness of inconsistent *outputs* is not necessarily a small *input* that reproduces the engine bug. Treat the two as different products.

**What exists already, and what is ours.**
- Already exists: NautilusTrader's own canonical result, BLAKE3 digest and `first_divergence()` for diffing two runs. We do not rebuild plain diffing. Generic test-case reducers exist, as do small ledger and evidence tools (LedgerX, backtest-evidence, PlainBacktest's independent ledger for PyBroker).
- Ours: (1) a field-semantics contract per engine, (2) independent fixed-point recomputation against verbatim observed claims, (3) a four-status verdict that refuses to call valid behaviour a bug, (4) a reducer that preserves lifecycles and fingerprints and never rewrites observed balances, and (5) evidence bundles you can replay.

**Done vs not done.** Nothing is implemented. There is no code, no benchmark and no corpus. Everything below is plan and specification.

**Main risks.** (1) False positives from misread field semantics; NautilusTrader #5483 was a case of this. (2) Upstream reports rejected for not meeting AI_POLICY.md, as happened with #5484. (3) Engines changing quickly, since Nautilus 2.0 is still in release candidates with breaking changes in each rc. (4) Precision loss on import. (5) Reducer slippage.

---

## PART B — Competitor check and corrected novelty wording

**dev-belly/LedgerX.** Python, stdlib-only core using `Decimal` with an isolated 128-digit half-even context. Cash, fees and balances are integers in units of 0.00000001 USD. Quantity and price allow up to 24 decimals. Its demo deposits 10,000 USD, buys 10 at 100 with a fee of 1, then sells 4 at 110 with a fee of 1, giving cash 9,438 and realised P&L 38.6. Boundaries are stated in the README: "USD, long-only and settled cash; no shorting, margin, FX or corporate actions." Input is its own string-typed JSONL event schema. It has batch ingest with idempotent retries but no NautilusTrader or LEAN adapter. It has no reducer, no invariant-violation taxonomy, and does not compare against an engine's own claims (it *is* the ledger). Activity is 12 commits and 1 star, and no licence appeared on the repo page, so treat it as all-rights-reserved until shown otherwise. **Overlap with QuantLab-RS:** the independent exact ledger idea. **No overlap:** external-engine verification and reduction.

**FanatiSam/backtest-evidence.** MIT, v0.1.0, a single commit, and it ships an AGENTS.md. Its inputs are a daily PnL CSV, an event CSV with timezone-aware timestamps, an observation-date CSV and a file allowlist for manifests. Its README says it does not "execute orders, obtain market data, model margin, validate fills…". The description of it as "independently recomputed backtest ledgers" is **wrong**. It has no reduction.

**Trace reducers.** GReduce (TOSEM) reduces *generator* traces to keep test inputs valid, not financial traces. I found no reducer for financial event traces.

**Corrected novelty wording to use publicly:** "To our knowledge (searches as of November 2026), no open tool combines engine-trace import, independent fixed-point recomputation of cash and positions, a non-accusatory verdict model, and fingerprint-preserving, lifecycle-valid reduction of financial inconsistencies into replayable fixtures. Related tools cover parts of this: LedgerX (exact research-fill journal), backtest-evidence (PnL/timestamp/manifest checks), and NautilusTrader's canonical result diffing."

**Verdict.** The flagship is defensible. The ledger alone is not novel: LedgerX and many others show that. The moat is reduction plus semantics contracts plus non-accusatory statuses.

---

## PART C — Verified November 2026 stack

Versions come from crates.io, docs.rs, GitHub or lib.rs as seen in searches. They may be a few weeks stale. **The agent must re-verify every version and pin it in Cargo.lock at CP1.0.**

| Need | Choice | Version seen (date) | Status / note |
|---|---|---|---|
| Toolchain | Rust stable, edition 2024 | 1.99.1 (15 October 2026). 1.100.0 due 12 November 2026 | Verified. Pin 1.99.1; move to 1.100.0 only after it is published, via an ADR |
| JSON boundary | serde + serde_json | 1.0.229 (18 July 2026); 1.0.154 (11 October 2026) | Verified |
| Fast JSON (optional) | sonic-rs behind a feature flag | 0.5.8 (25 March 2026), with conflicting evidence for 0.5.10 | Re-verify; adopt only if measured faster on our records |
| Mapped input | memmap2 | 0.9.11 (22 June 2026) | Verified |
| Parallelism | rayon | 1.12.0 (April 2026) | Verified, though a newer release can't be ruled out |
| Arena | bumpalo (optional) | 3.20.3 (22 May 2026) | Verified, possibly stale |
| Small vectors | smallvec 1.x stable | 1.16.1 (11 September 2026) | Do not use the 2.0.0-beta |
| Content hash | blake3 | 1.8.7 (20 August 2026) | Verified. Nautilus also uses BLAKE3 for result digests |
| In-memory hash | foldhash (alternative: rustc-hash) | 0.2.0 / 2.1.3 | Dates unverified |
| Allocator (CLI binary only, optional) | mimalloc | 0.1.52 (22 May 2026) | Verified. Measure before enabling |
| CLI | clap | 4.6.6 (6 August 2026) | Verified |
| Errors | thiserror | 2.0.21 (23 September 2026) | Verified |
| Logging | tracing | 0.1.44 seen | Version and date unverified |
| Property tests | proptest | 1.11.0 (24 March 2026) | Verified |
| Microbench | criterion | 0.8.2 seen | Unverified date |
| Alt bench | divan | 0.1.21 seen | Unverified; evaluate only |
| Instruction-count bench | gungraun (formerly iai-callgrind) | 0.20.0 (26 September 2026) | Verified. Valgrind-based, so use on Linux CI |
| Test runner | cargo-nextest | 0.9.143 (4 August 2026, via lib.rs) | Verified via a mirror |
| Mutation | cargo-mutants | 27.1.0 (2 June 2026) | Verified |
| Licence/advisories | cargo-deny, cargo-audit | 0.19.9 (15 June 2026); 0.22.2 (5 June 2026) | Verified |
| Fuzz | cargo-fuzz | not verified | Part 3; nightly toolchain |
| UB check | Miri (nightly component) | n/a | For `unsafe` (ideally none) and arithmetic edge cases |
| 256-bit widening | Custom minimal U256 mul plus division by 10^k, oracle-tested (alternatives ruint or bnum) | ruint/bnum not verified | std `widening_mul` is still unstable (tracking #152016, changed to return a single wider scalar in May 2026). Unsigned "basics" were partially stabilised in 1.91.0 (#152015); the agent must check whether `u128::carrying_mul` is stable on the pinned toolchain |
| Dependency graph | Custom CSR (u32 offsets and targets) | n/a | petgraph rejected for the hot path; allowed in tests |
| Bitsets | Custom `Vec<u64>` alive mask | n/a | fixedbitset/roaring unverified; roaring is a Part 2/3 candidate for sparse huge traces |
| Fixed-point crates | Custom `qlrs-num` | n/a | rust_decimal (used by Nautilus) is a 96-bit mantissa decimal, not our exact i128-at-declared-scale model; it is allowed only as a test oracle |

**Rejected or deferred:** simd-json (last version seen 0.17.0, October 2025, and it needs mutable buffers; re-check before ever adopting), Arrow/Parquet (moved to Part 6), SQLite (Part 11), any float in money paths, petgraph in hot paths, global `target-cpu=native` in distributed builds.

---

## PART D — Roadmap (17 parts, dependency-ordered)

| # | Part | Goal | Key deliverables | Exit gate |
|---|---|---|---|---|
| 1 | Counterexample Engine v0.1 (thin slice) | Prove the full loop on synthetic traces | num core, schema, columnar trace store, ledger, I1–I3, statuses, fingerprints, reducer v0, bundle v0, planted defects, benchmarks | **Gate A-synthetic**; early B-synthetic on ≥3 defects |
| 2 | Dependency-aware reduction v1 | Flagship algorithm | Binary reduction over dependency closures, algebraic fast evaluator, parallel speculative evaluation, slippage guards, 1-minimality certificate | **Gate B-synthetic** (all reducible defects) |
| 3 | Scale and hardening | 10^6–10^7-event synthetic generator, fuzzing, mutation ratchet, measured baselines | Generator, cargo-fuzz targets, memory budget report | Perf baseline committed; no fuzz crashes in 1 CPU-hour per target |
| 4 | NautilusTrader field-semantics contract and corpus triage | Know what each field means before importing | Written contract at a pinned version (2.0.0rcN or final); corpus cases with status (confirmed bug / expected / already fixed / not detectable); #5483 and #5484 recorded | Human sign-off on the contract |
| 5 | NautilusTrader adapter (Mode T) | Lossless import of real runs | Reader for `CanonicalBacktestResult` v1 plus event streams; precision mapping; CONSISTENT on clean real runs | **Gate A (real)** |
| 6 | Columnar storage | Large real traces | Arrow/Parquet ingest, mmap'd column store, real-trace perf | Measured throughput at real scale |
| 7 | Model expansion I | More realistic spot accounting | Multi-currency, commission currency ≠ quote, limit/stop orders, settlement | Invariants extended without new false positives |
| 8 | Evidence bundle v1 and financial semantic diff | Reviewable evidence | Bundle v1, diff layered *on top of* Nautilus `first_divergence` | Bundle verifies offline |
| 9 | CLI v1 and CI verification | Usable in pipelines | `qlrs` release, CI action, JUnit/SARIF | **Gate B (real)** on Nautilus traces |
| 10 | Upstream reproduction workflow | First external use | Human-reproduced case on current `develop`, AI_POLICY-compliant report | **Gate C attempt** |
| 11 | Viewer backend | Local API | Axum server, SQLite metadata | API contract tests |
| 12 | Viewer frontend I | React + TS + Vite | Overview, Import, Ledger Explorer, Violations | E2E tests |
| 13 | Viewer frontend II | Explain reductions | Counterexample Explorer, Evidence Reports | E2E tests |
| 14 | LEAN adapter | Second engine | LEAN field contract; #9819 and #9838 as corpus candidates | Gate A for LEAN; Gate C re-attempt |
| 15 | Model expansion II | Harder finance | Margin, derivatives, dividends, delistings, splits | No false positives on clean corpora |
| 16 | Mode E engine-rerun minimiser | Input reduction | Engine harness (Nautilus first), input-level reduction | Reduced input reproduces the fingerprint |
| 17 | 1.0 hardening | Ship | PGO, packaging, docs, governance | Release |

---

## PART E — `PART_01_INSTRUCTIONS.md` (hand this whole section to the agent)

### 0. Mission

Build QuantLab-RS Part 1. This is a Rust workspace that imports synthetic backtest traces (JSONL), replays them on an independent fixed-point ledger, evaluates three financial invariants, assigns one of four statuses, fingerprints violations, reduces violating traces to small lifecycle-valid witnesses, and emits a deterministic evidence bundle. Performance is a requirement from day one: JSONL is used only at the I/O boundary, and all replay, checking and reduction run on a compact columnar in-memory store.

### 1. Scope and non-goals

**In scope:** one cash account; one currency, which is both the account currency and the commission currency; spot instruments quoted in that currency; MARKET orders only; partial fills; non-negative fees; observed claims for account balance (total) and position quantity; invariants I1–I3; statuses; fingerprints; reducer v0; bundle v0; CLI with `verify`, `reduce` and `bundle verify`; benchmarks.

**Non-goals (any of these appearing in input gives UNSUPPORTED, never VIOLATION):** multiple currencies or FX; commission currency ≠ account currency; margin, leverage or shorting; limit, stop and other order types; corporate actions; funding; settlement delays; negative fees or rebates; NautilusTrader or LEAN import; Arrow/Parquet; SQLite; web UI; Mode E.

### 2. Assumptions

- Input traces are synthetic and produced by our own generator or hand-written fixtures.
- Each trace declares its financial model in a header. If an event falls outside that model, the result is UNSUPPORTED.
- Observed claims (engine-reported balances and positions) are evidence. They are never modified.

### 3. Target environment

- Primary development machine: macOS on Apple Silicon (aarch64-apple-darwin).
- CI: Linux x86_64 plus macOS arm64.
- Toolchain: pin `1.99.1` in `rust-toolchain.toml` with components rustfmt and clippy. Use a separate nightly only for Miri and fuzzing.
- Do not set `target-cpu=native` in committed config. A local-only `.cargo/config.toml` override may be documented for benchmarking.

### 4. Mandatory agent rules

1. **No unverified versions.** Before adding any dependency, check crates.io and docs.rs for its current version and maintenance (a release within roughly 12 months, or explicit maintainer statement). Record version, date and URL in `docs/STACK.md`. Use `--locked` everywhere.
2. **No benchmark claims without running.** Every number in any document must come from a committed raw output file under `bench-results/<date>-<machine>/`, together with `rustc -Vv`, CPU model, OS version and the command used. Without a file, write "not measured".
3. **ADRs.** Record each non-trivial decision in `docs/adr/NNNN-title.md` (context, options, decision, consequences). ADR-0001 (numeric representation), ADR-0002 (event layout) and ADR-0003 (reduction predicate) are mandatory in Part 1.
4. **Never weaken tests or tolerances to pass.** Do not edit golden files, expected statuses, the defect catalogue, rounding policy, or tolerance settings to make a failing test pass. If the spec looks wrong, stop and report.
5. **Stop and report on blockers.** If a blocker lasts more than two attempts, write `docs/BLOCKERS.md` (symptom, what was tried, hypothesis) and stop that checkpoint.
6. **No floats in money paths.** `f32` and `f64` are forbidden in `qlrs-num`, `qlrs-schema`, `qlrs-ledger`, `qlrs-check` and `qlrs-reduce`. Enforce with clippy `float_arithmetic` (deny) in those crates and a CI grep for `f32|f64` in their `src/`.
7. **No fabricated results.** Do not claim a test passes, a bug is found, or a reduction happened unless a command output shows it.
8. **No `unsafe`** in Part 1 (`#ǃ[forbid(unsafe_code)]` in every crate). Any exception needs an ADR plus Miri coverage.
9. **Determinism.** The same input gives byte-identical output regardless of thread count, OS or run.

### 5. Repository and crate layout

```
quantlab-rs/
  AGENTS.md  rust-toolchain.toml  Cargo.toml (workspace)  deny.toml  .config/nextest.toml
  docs/{SPEC.md, STACK.md, adr/, BLOCKERS.md, HANDOFF_PART_01.md}
  crates/
    qlrs-num      fixed-point decimal, rounding, checked/wide arithmetic (no deps beyond std)
    qlrs-schema   event/claim/header types, IDs, model profile, statuses (depends: num)
    qlrs-io       JSONL reader/writer, interner, TraceStore builder (depends: schema, num)
    qlrs-ledger   independent replay, expected state, per-event deltas (depends: schema, num)
    qlrs-check    invariants I1–I3, model preconditions, verdicts, fingerprints (depends: ledger)
    qlrs-reduce   units, dependency closure, predicate, reducer, cache (depends: check)
    qlrs-bundle   bundle v0 writer/verifier (depends: reduce, io)
    qlrs-cli      binary `qlrs` (depends: all; only crate allowed clap/tracing-subscriber/mimalloc)
    qlrs-testkit  fixtures, generator, defect injector, oracles (dev-only)
    qlrs-bench    criterion (+ gungraun on Linux) benches
  fixtures/{clean/, defects/, golden/}  bench-results/
```

**Dependency direction:** num ← schema ← {io, ledger} ← check ← reduce ← bundle ← cli. No cycles. Only `qlrs-io` may depend on serde_json or sonic-rs. `qlrs-ledger` and `qlrs-check` must not depend on `qlrs-io`; they see only `TraceStore`. CI checks this with `cargo tree` assertions in a script.

### 6. Pinned stack (re-verify at CP1.0)

Use the PART C table. Runtime dependencies allowed in Part 1: serde, serde_json, (sonic-rs optional feature `fast-json`), memmap2, rayon, smallvec, blake3, foldhash, thiserror, clap, tracing, (mimalloc optional feature `mimalloc` in cli). Dev dependencies: proptest, criterion, gungraun (Linux), plus one arbitrary-precision integer crate used only as a test oracle (choose and verify it, record in STACK.md). Adding anything else requires an ADR.

### 7. Cargo profiles, lints, CI, AGENTS.md

**Profiles (workspace Cargo.toml):**
- `[profile.release]`: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` (binary only; tests still use unwinding via the test profile), `debug = "line-tables-only"` for profiling symbols, `overflow-checks = true` in Part 1 (we use explicit checked ops anyway; measure the cost at CP1.8 and decide by ADR).
- `[profile.bench]`: inherits release, with `panic = "unwind"` kept for harness compatibility.
- `[profile.dev.package."*"]`: `opt-level = 2` so property tests run fast.
- PGO is deferred to Part 17. Record it in an ADR as future work.

**Lints (`[workspace.lints]`):** rust `unsafe_code = "forbid"`, `missing_docs = "warn"`; clippy `pedantic` warn, plus deny on `float_arithmetic` (money crates), `cast_possible_truncation`, `cast_sign_loss`, `arithmetic_side_effects` (in num, ledger, check), `unwrap_used` and `expect_used` (non-test), `indexing_slicing` (warn, allowed in hot loops only with a `// SAFETY-BOUNDS:` comment justifying it).

**CI jobs (all must pass):** fmt check; clippy `-D warnings`; nextest on both OSes; doc tests; cargo-deny (licences, bans, advisories, sources); cargo-audit; dependency-direction script; float-ban grep; golden-manifest check; determinism job (run `qlrs verify` and `qlrs reduce` with `RAYON_NUM_THREADS=1` and `=8`, then diff bundle bytes); Miri on `qlrs-num` (nightly); mutation testing on `qlrs-num` and `qlrs-check` (scheduled or manual, ratchet only).

**AGENTS.md content (create at CP1.0):** project one-liner; build, test and bench commands; the rules in §4 verbatim; protected paths (`fixtures/golden/**`, `fixtures/defects/catalogue.toml`, `docs/SPEC.md` requirement IDs, `deny.toml`) which no change may touch without an ADR and human approval; definition of done for a checkpoint; commit message format `cp1.N: <summary>`.

**Anti-weakening mechanics:**
- `fixtures/golden/MANIFEST.blake3` lists the BLAKE3 hash of every golden file, and CI recomputes them.
- A test-count ratchet stored in `ci/test-count.txt` may only increase.
- A mutation-score ratchet in `ci/mutants-baseline.txt`.
- Every spec requirement has an ID (`REQ-NUM-003`, etc.), and `docs/TRACE.md` maps requirement to test names. CI fails if any REQ ID has no test.

### 8. Domain specification

#### 8.1 Numeric representation (ADR-0001)

- **Type:** `Fixed { raw: i128, scale: u8 }` with value = raw × 10^(−scale). `scale ≤ 18` is supported in Part 1 to leave room for crypto assets with 18 decimals; anything larger gives UNSUPPORTED.
- **Scale is per value, not a global 10^-9.** The header declares `currency.precision`, `instrument.qty_precision` and `instrument.px_precision`. Values must not carry more digits than declared, **except commissions**, which may carry up to `commission_max_scale` (header, default = currency precision). This mirrors NautilusTrader 2.0.0rc8, which accumulates fractional Money commissions beyond currency precision.
- **Parsing:** decimal strings only (`"-123.4500"`); no exponents, no leading `+`, no NaN or infinity. Trailing zeros are allowed up to the declared scale. JSON numbers for money are rejected.
- **Exactness first:** notional = last_qty × last_px is computed exactly at scale `qty_scale + px_scale`. Comparisons with claims use **upscaling** (multiplying the claim by 10^k), never downscaling, so the hot path does no division.
- **Rounding is used only when the model profile declares it.** `engine_cash_rounding ∈ {Exact, HalfEven(currency_precision), HalfUp(...), Down(...)}`. `Exact` means an inexact representable cash change gives INCONCLUSIVE with reason `rounding-policy-required`.
- **Overflow:** every add, sub and mul is checked. A mul overflow in i128 falls back to a cold 256-bit path (U256 product, then exact division by 10^k with remainder for rounding). A result that does not fit i128 at the target scale gives INCONCLUSIVE with reason `numeric-overflow`. The process never panics on overflow.
- **Ranges:** document that NautilusTrader high-precision Price/Money is i128 at 16 decimals, with a range of −17,014,118,346,046 to 17,014,118,346,046 per the NautilusTrader Concepts overview, and Quantity is u128 with a range of 0 to 34,028,236,692,093. Our i128 at a declared scale is a superset for import as long as scale ≤ 18; check this formally in Part 4.

#### 8.2 Trace file format (JSONL, `qlrs-trace/v0`)

Line 1 is the header; each later line is one record. UTF-8, `\n`-terminated, no BOM. Unknown fields cause rejection (strict mode).

**Header record fields**

| Field | Type | Notes |
|---|---|---|
| `type` | `"header"` | |
| `schema` | `"qlrs-trace/v0"` | |
| `model` | object | `{account_type:"CASH", base_currency, engine_cash_rounding, commission_max_scale}` |
| `currencies` | array | `{code: str(3–8), precision: u8}`; exactly one entry in Part 1 |
| `instruments` | array | `{id, quote_currency, qty_precision, px_precision}` |
| `accounts` | array | `{account_id, starting_balance: decimal-string}` (one in Part 1) |
| `origin` | object | `{producer, producer_version, generated_by_qlrs: bool}` |

**Event records (common fields):** `seq` (u64, strictly increasing, unique), `ts_event` (u64 Unix ns, non-decreasing in Part 1), `type`, `account_id`, `provenance` (`"observed"` | `"synthetic"`; input traces must be all observed).

| `type` | Extra fields | Nautilus analogue (for Part 4; re-verify) |
|---|---|---|
| `OrderSubmitted` | `client_order_id`, `instrument_id`, `side` (BUY/SELL), `order_type` (MARKET), `quantity` | OrderSubmitted / order init |
| `OrderAccepted` | `client_order_id`, `venue_order_id` | OrderAccepted |
| `OrderRejected` | `client_order_id`, `reason` | OrderRejected |
| `OrderFilled` | `client_order_id`, `trade_id`, `last_qty`, `last_px`, `commission`, `commission_currency`, `liquidity_side` (MAKER/TAKER/NONE) | OrderFilled (`trade_id`, `last_qty`, `last_px`, `currency`, `commission: Option<Money>`, `liquidity_side`, `ts_event`, `ts_init`, `position_id`) |
| `OrderCanceled` | `client_order_id` | OrderCanceled |
| `AccountClaim` | `currency`, `balance_total` | AccountState balances (verify total/locked/free) |
| `PositionClaim` | `instrument_id`, `net_qty` (signed decimal) | Position / position snapshot (`quantity` describes current exposure; a closed position is FLAT/0, per #5483) |

**In-memory layout (ADR-0002):**
- IDs are interned to `u32` at parse time (foldhash map to dense indices; strings stored in a single byte arena with offsets).
- The `TraceStore` is columnar **per kind**:
  - an event spine with SoA columns `seq: Vec<u64>`, `ts: Vec<u64>`, `kind: Vec<u8>`, `unit: Vec<u32>` and `payload_idx: Vec<u32>`;
  - kind tables such as `fills {order, trade, qty_raw: i128, px_raw: i128, fee_raw: i128, ...}`, `claims_cash {currency, value_raw}`, `claims_pos {...}` and `orders {...}`.
  - Scales are stored per instrument or currency in small side tables, with commissions carrying their own scale column.
  - Per-event cash deltas are precomputed once into `delta_cash: Vec<i128>` at a common account scale.
- Estimated footprint is about 40–60 bytes per event (an estimate; measure at CP1.8).

#### 8.3 Statuses

| Status | Meaning | Exit code |
|---|---|---|
| `CONSISTENT` | No contradiction found under the declared model. This is not a proof of correctness | 0 |
| `VIOLATION` | At least one invariant has a nonzero residual under the declared model, with complete evidence | 2 |
| `INCONCLUSIVE` | Evidence or assumptions are missing (no claim to compare, a rounding policy is required, numeric overflow) | 3 |
| `UNSUPPORTED` | Input is outside the declared model (second currency, short position, LIMIT order, negative fee, etc.) | 4 |
| (input error) | Malformed file | 1 |

Precedence for the overall verdict: input error, then UNSUPPORTED (if any event is unsupported, no VIOLATION is reported unless the user explicitly requests `--partial`), then VIOLATION, then INCONCLUSIVE, then CONSISTENT.

#### 8.4 Invariants (mathematical)

Notation: events e₁…eₙ in `seq` order. δ(e) is the expected cash change: for a BUY fill, δ = −(q·p) − f; for a SELL fill, δ = +(q·p) − f; for all other kinds, δ = 0. π_i(e) is the expected position change for instrument i: +q for BUY and −q for SELL fills of i.

- **I1 cash conservation per fill (local window form).** Take consecutive observed AccountClaims c_{j−1} and c_j for the same account and currency. The first claim's predecessor is the declared starting balance, or a SYNTHETIC checkpoint. The residual is r_j = (obs(c_j) − obs(c_{j−1})) − Σ_{e ∈ (c_{j−1}, c_j)} δ(e), with rounding applied per the model profile. **VIOLATION iff r_j ≠ 0.** This local form is *removal-stable* for units entirely outside the window, which is what makes reduction possible. The global form ("final cash = start + Σδ") is reported only as a derived summary.
- **I2 position equals sum of fills.** For each PositionClaim k on instrument i, expected position = P₀(i) + Σ_{e < k} π_i(e), where P₀ is 0 or a SYNTHETIC checkpoint value. The residual is obs − expected. VIOLATION iff the residual ≠ 0. A flat position (0) after a close is CONSISTENT: never flag "position closed so quantity is 0" (#5483).
- **I3 lifecycle legality.** Per `client_order_id`, the event sequence must be accepted by this state machine: `∅ →Submitted→ S`, `S →Accepted→ A`, `S →Rejected→ R(terminal)`, `A|PF →Filled(partial)→ PF`, `A|PF →Filled(completing)→ F(terminal)`, `A|PF →Canceled→ C(terminal)`. Also Σ last_qty ≤ quantity, `trade_id` is unique per instrument, and there are no events after a terminal state. Any breach is a VIOLATION with a sub-code (`fill-before-accept`, `overfill`, `event-after-terminal`, `duplicate-trade-id`, `unknown-order`).
- **Model preconditions (give UNSUPPORTED, not VIOLATION):** an expected position going negative (shorting); expected cash going negative (overspend — deferred to Part 7 with a field contract); any out-of-model event.

The Part 1 state machine is a deliberate subset. Part 7 extends it toward NautilusTrader's richer states (e.g. INITIALIZED, DENIED, EMULATED, RELEASED, TRIGGERED, PENDING_UPDATE, PENDING_CANCEL, EXPIRED) and FIX OrdStatus (tag 39). Keep the enum `#[non_exhaustive]` and the transition table data-driven so it can grow.

#### 8.5 Fingerprints

- **Instance key (reduction predicate identity):** `(invariant_id, sub_code, account, currency|instrument, anchor_claim_original_seq, residual_raw, residual_scale)`. Reduction never renames IDs or seqs, so this key stays comparable.
- **Class fingerprint (triage and dedup across traces):** `qlrs-fp1:` + BLAKE3 over canonical bytes of `(invariant_id, sub_code, kind pattern of the window as a sorted multiset, residual sign, residual relation)`. Residual relation is one of: `= +fee`, `= −fee`, `= ±k·last_px` (small integer k ≤ 10), `= ±notional`, `= ±10^m·expected` (scale error), or `other`. Document that the relation is a heuristic hint, not a diagnosis.
- Canonical bytes: fixed field order, length-prefixed strings, little-endian integers. Spec this in SPEC.md with a test vector.

#### 8.6 Tolerance policy

Default tolerance is zero. The only accepted non-exact comparison is "the engine emits rounded values". That must be declared in the header as `engine_cash_rounding`, and the expected value is then rounded the same way before comparison. Never use an absolute epsilon. Any other mismatch is a residual.

#### 8.7 Planted defect catalogue (`fixtures/defects/catalogue.toml`, protected)

Each defect is a mutation operator applied to a clean trace by `qlrs-testkit`, with an expected status and invariant.

| ID | Mutation | Expected |
|---|---|---|
| D01 | Fee charged twice in the next AccountClaim | VIOLATION I1, relation −fee |
| D02 | Fee omitted | VIOLATION I1, +fee |
| D03 | Notional off by one share | VIOLATION I1, ±1·last_px |
| D04 | SELL credited as debit | VIOLATION I1, ±2·notional (relation `other` permitted) |
| D05 | Fill applied twice to cash (claim double counts) | VIOLATION I1 |
| D06 | Partial fill not reflected in PositionClaim | VIOLATION I2 |
| D07 | Fill after Canceled | VIOLATION I3 event-after-terminal |
| D08 | Overfill | VIOLATION I3 overfill |
| D09 | Fill before Accepted | VIOLATION I3 fill-before-accept |
| D10 | Claim scaled ×10 | VIOLATION I1, scale-error relation |
| D11 | Duplicate trade_id | VIOLATION I3 duplicate-trade-id |
| D12 | Commission in a second currency | **UNSUPPORTED** (must not be VIOLATION) |
| D13 | SELL exceeding position | **UNSUPPORTED** (shorting outside model) |
| D14 | Inexact cash change with `Exact` rounding | **INCONCLUSIVE** |

**Clean fixtures (must be CONSISTENT):** C01 is the ₹10,000 worked example with the correct claim ₹7,990.00. Others cover partial fills; zero fee; cash exactly 0 after a buy; buy then sell-all giving PositionClaim 0 (FLAT); 18-decimal quantities; fees with more digits than currency precision (within `commission_max_scale`); 10^6-event generated traces.

#### 8.8 Bundle v0 (`qlrs-bundle/v0`, a directory)

| File | Content |
|---|---|
| `manifest.json` | schema, qlrs version, git commit, rustc version, model profile, verdict, list of files with BLAKE3 hashes, original trace BLAKE3 and byte length |
| `violation.json` | instance key, class fingerprint, invariant, residual, expected (computed) and observed values **as verbatim strings**, window seqs |
| `reduced_trace.jsonl` | valid `qlrs-trace/v0`; observed records byte-identical to the original lines |
| `synthetic_checkpoint.json` | present only if history was removed: `provenance:"synthetic"`, state (cash, positions), cut seq, "computed by qlrs ledger from original trace <digest>" |
| `reduction_log.jsonl` | one line per predicate call: candidate digest, units removed, result, cache hit |
| `REPRODUCE.md` | exact commands (`qlrs verify reduced_trace.jsonl --checkpoint ...`) |

`qlrs bundle verify <dir>` recomputes all hashes, re-runs verification on the reduced trace, and asserts that the same instance key is found.

### 9. Reduction specification (Part 1 = v0; Part 2 extends it)

**Reduction unit.** One **lifecycle unit** is all events with the same `client_order_id`. Each observed claim is its own **claim unit**. The header and instruments are pinned.

**Pinned set.** The two claims that bound the violating window (or, for I2/I3, the violating claim or order) plus the lifecycle units with any event inside the window are never removable in v0.

**Dependencies (closure).** A SELL lifecycle depends on earlier BUY lifecycles of the same instrument that are needed to keep the expected position ≥ 0. Compute them conservatively: all earlier BUYs until the cumulative quantity covers the sell. Removal works on **closures**: removing a unit also removes every unit that depends on it. The dependency graph is stored as CSR.

**Synthetic checkpoint.** If every event before seq s is removed, insert a checkpoint whose state equals the **independently computed expected state at s from the full original trace**. Label it `provenance:"synthetic"`. Observed claims are never altered, and observed values are never used to build the checkpoint.

**Predicate P(candidate)** holds iff all of the following are true:
- (a) the replay is lifecycle-valid (no I3 breach other than the target);
- (b) the target instance key is reproduced exactly (same invariant, anchor claim seq and residual);
- (c) no UNSUPPORTED or INCONCLUSIVE condition arises;
- (d) no *new* violation instance appears that was absent from the original (the anti-slippage rule; configurable to "allow pre-existing ones").

**Algorithm v0.**
1. Stage A slices with O(1) predicate calls: remove every unit that is not pinned and not a dependency of a pinned unit, place the synthetic checkpoint, then check P. Because I1 is local, this usually succeeds immediately.
2. Stage B runs ddmin (Zeller & Hildebrandt) over the remaining removable closures in seq order.
3. Stage C makes a final 1-minimality pass: try removing each remaining non-pinned closure singly.

**Guarantee obtained:** *closure-1-minimal* relative to the unit and closure definition and the predicate. No single remaining removable closure can be deleted while keeping P. This is **not** a global minimum, and the report must say so.

**Speed.**
- **Predicate cache:** a foldhash set of BLAKE3 hashes of the candidate's sorted kept-unit bitset.
- **Algebraic fast evaluator:** for I1, removing units outside the window changes nothing, and inside-window units are pinned, so P can be decided from precomputed per-unit contributions. Position checks use prefix sums.
- **Full replay of the final witness is always run.** If the fast and full results disagree, that is a bug: stop and report.
- **Parallel speculative evaluation:** evaluate all ddmin subsets and complements of one round in parallel with rayon, then accept the **lowest-index** success so results do not depend on thread count.

**Complexity:**
- ddmin worst case is O(n²) predicate calls over n closures. Stage A makes the typical case close to O(1).
- Binary reduction over dependency graphs (Kalhauge & Palsberg, ESEC/FSE 2019) is the Part 2 upgrade. Kalhauge and Palsberg report that their J-Reduce tool "is 12x faster and achieves more reduction than delta debugging on average" on Java bytecode.
- ProbDD-style counters and WDD weights are also Part 2 candidates. Zhang et al. (ICSE 2025) found that "the probabilities in ProbDD essentially serve as monotonically increasing counters", and WDD (Zhou et al., ICSE 2025) assigns "each element in the list a weight according to its size"; with W_ddmin, HDD took 51.31% less time to produce 9.12% smaller results than with ddmin.

### 10. Performance requirements and techniques

| Area | Technique | Where |
|---|---|---|
| Parsing | mmap the file; split into line-aligned chunks; parse in parallel with borrowed `&str` deserialisation; intern IDs; merge chunks in order | qlrs-io |
| Decimals | Hand-written decimal-string to i128 parser (no float, no allocation), with scale validation | qlrs-num |
| Arithmetic | Hot path in i128 checked ops; `#[cold]` 256-bit fallback; no division in the hot path (compare by upscaling); division by 10^k dispatched through a `match k` onto const-generic specialisations when rounding is required | qlrs-num, ledger |
| Layout | Per-kind SoA tables; dense u32 IDs; precomputed `delta_cash` column | io, ledger |
| Replay | Zero allocations in the replay loop (assert with a counting allocator in a test); state is a small fixed array indexed by dense currency or instrument ID | ledger |
| Reduction | Bitset alive-mask (`Vec<u64>`, iterate with `trailing_zeros`); CSR graphs; cache; algebraic evaluator; prefix snapshots every K events for full replay from the nearest snapshot | reduce |
| Hashing | BLAKE3 for content; foldhash for maps | all |
| Allocator | mimalloc only in the CLI binary, behind a feature, adopted only if measured faster | cli |

**Targets (ESTIMATES, to be replaced by measurements at CP1.8):**
- Replay of at least 50 M events/s single-threaded on an M-series core for fill-heavy traces.
- JSONL parse of at least 300 MB/s across all cores.
- Reduction of a 10^6-event trace with one planted I1 defect to ≤ 6 events in under 2 s.
- Peak memory ≤ 100 bytes/event.

**Benchmark methodology:**
- criterion on macOS and Linux, with fixed inputs generated with a seed and committed digests.
- Report median and confidence intervals; warm-up defaults; machine info file.
- gungraun instruction counts on Linux CI as the regression gate.
- Gate: no more than 5% instruction-count regression against the committed baseline without an ADR.

### 11. Testing strategy

- **Unit tests** per REQ ID.
- **Property tests (proptest):**
  - Decimal parse then format round-trips.
  - Fixed-point mul and rescale match the big-int oracle for random raw values and scales 0–18, including values near i128 limits.
  - Checked ops never panic.
  - Every clean generated trace is CONSISTENT.
  - For every defect operator applied at a random position, the status equals the catalogue entry.
  - The reduced trace satisfies P, is a subsequence of the original plus at most one synthetic record, has byte-identical observed lines, and is closure-1-minimal.
  - Reduction is idempotent (reducing a reduced trace yields the same result).
  - Output is invariant under thread count.
- **Integration tests:** CLI exit codes, golden bundles.
- **Mutation testing:** `cargo mutants` on num and check; record the baseline and do not let it decrease.
- **Miri:** `qlrs-num` tests.
- **Determinism:** two runs and two thread counts give identical bytes.

### 12. Checkpoints

Every checkpoint must pass these verification commands:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo nextest run --workspace --locked`
- `cargo test --doc --workspace --locked`
- `cargo deny check`
- `cargo audit`
- `./ci/check-deps.sh`
- `./ci/float-ban.sh`
- `./ci/golden-manifest.sh`

The commit format is `cp1.N: <summary>`, plus a tag `cp1.N`. **Stop and report** if any criterion fails after two honest attempts.

**CP1.0 — Bootstrap and guardrails.**
- Tasks: workspace, toolchain pin, AGENTS.md, SPEC.md skeleton with REQ IDs, STACK.md (versions re-verified), ADR-0001/0002/0003 drafts, deny.toml, lints, CI workflows, ratchet files, empty crates honouring the dependency direction.
- Pass when: CI is green on both OSes; `check-deps.sh` fails when a forbidden edge is added deliberately (test this, then revert); STACK.md lists every dependency with URL and date.
- Stop if: any crate version cannot be verified.

**CP1.1 — Numeric core (`qlrs-num`).**
- Tasks: `Fixed`, parser and formatter, checked add/sub/mul, mul-rescale with 256-bit cold path, rounding modes, error enum.
- Pass when: property tests against the oracle pass for 100k cases per property; Miri is clean; no panics in fuzz-like proptest of the parser; benches exist for parse and mul.
- Stop if: the oracle disagrees on any rounding case.

**CP1.2 — Schema, JSONL boundary and TraceStore.**
- Tasks: header and record types, strict parser (unknown field means reject), interner, per-kind SoA store, writer that reproduces observed lines byte-identically.
- Pass when: malformed-input tests return exit 1 with line numbers; 10^6-event parse succeeds; the round-trip writer is byte-identical.

**CP1.3 — Independent ledger.**
- Tasks: delta precomputation, expected cash and positions, window sums, zero-allocation replay.
- Pass when: C01 expected cash is exactly ₹7,990.00; the counting-allocator test shows 0 allocations in the replay loop.

**CP1.4 — Invariants and statuses.**
- Tasks: I1, I2, I3, preconditions, verdict precedence, exit codes.
- Pass when: all clean fixtures are CONSISTENT; D12 is UNSUPPORTED; D13 is UNSUPPORTED; D14 is INCONCLUSIVE.

**CP1.5 — Fingerprints and defect corpus.**
- Tasks: instance key, class fingerprint with test vector, defect injector, generator with seeds.
- Pass when: **Gate A-synthetic**. All D01–D14 give their expected status at 1,000 random injection points each; 0 false VIOLATIONs on 1,000 clean generated traces.

**CP1.6 — Reducer v0.**
- Tasks: units, CSR dependencies, closures, synthetic checkpoint, predicate with anti-slippage rule, Stage A/B/C, cache, parallel evaluation, fast-versus-full cross-check.
- Pass when: **early Gate B-synthetic**. For D01, D03 and D07 injected into 10^6-event traces, the reduced witness satisfies P, keeps observed lines byte-identical, labels the checkpoint synthetic, and is closure-1-minimal; the reduction log records every call.

**CP1.7 — Bundle v0 and CLI.**
- Tasks: `qlrs verify|reduce|bundle verify`, bundle writer and verifier, REPRODUCE.md.
- Pass when: `bundle verify` succeeds on generated bundles and fails if any byte changes (tamper test); the determinism job is green.

**CP1.8 — Measure, harden, hand off.**
- Tasks: run all benches, commit raw results, run cargo-mutants and record baselines, decide `overflow-checks` by ADR, write the HANDOFF report.
- Pass when: every number in HANDOFF has a raw file; the test-count and mutation ratchets are committed.

### 13. Final handoff report template (`docs/HANDOFF_PART_01.md`)

Sections:
1. Done (per checkpoint, with commit hashes)
2. Not done
3. Measured numbers (with links to raw files and machine info)
4. Deviations from this spec (with ADR links)
5. Known issues and risks
6. Gate results (A-synthetic matrix, B-synthetic cases)
7. What Part 2 can rely on (stable interfaces, below)
8. Open questions for the human

### 14. Definition of Done (Part 1)

- All CP1.0–CP1.8 criteria pass in CI.
- Gate A-synthetic holds: the expected status for every catalogue entry, and 0 false VIOLATIONs on clean corpora.
- Early B-synthetic holds on ≥3 defects.
- No floats or `unsafe` in money crates.
- Bundles are deterministic.
- HANDOFF is complete with measured numbers only.

### 15. Interfaces Part 2 depends on (keep stable; changes need an ADR)

- `TraceStore` (columns, dense IDs, unit index, `delta_cash`).
- `Ledger::replay(store, alive_mask, checkpoint) -> ExpectedState`.
- `Checker::evaluate(store, expected) -> Vec<Finding>` with `Finding { status, instance_key, class_fp, residual, window }`.
- `Predicate` trait: `test(alive_mask) -> Outcome`.
- `UnitGraph` (CSR) and closure API.
- `SyntheticCheckpoint` type.
- The `qlrs-trace/v0` and `qlrs-bundle/v0` schemas.
- The defect catalogue format.

---

## Recommendations

1. Treat this as the final project. Lock the public novelty wording given in PART B.
2. Pin NautilusTrader work (Part 4 onward) to a specific rc or the 2.0.0 final once it exists. Every rc so far has shipped breaking changes, including rc8's changes to Money precision.
3. Before any upstream report, the human must reproduce it on current `develop`, check field semantics against the contract, and follow AI_POLICY.md. I could not fetch AI_POLICY.md itself. CONTRIBUTING.md states that AI users "remain responsible for every submission", that the policy covers "human direction, review, communication, and attribution", and that PR authors must "understand and can explain every submitted change". Read the file directly before Part 10.
4. Add LEAN #9819 (and #9838) to the Part 14 corpus as "candidate confirmed bug — fix in flight".

## Caveats

- Crate versions came partly from mirrors and search snapshots. Several (tracing, criterion, divan, petgraph, simd-json, ruint, bnum, fixedbitset, roaring, cargo-fuzz) are unverified. CP1.0 requires re-verification.
- Rust 1.100.0 has not shipped yet (it is due 12 November 2026). The name of its Cargo cooldown config key is unverified.
- I did not confirm whether LEAN PR #9878 has merged.
- Contents of `CanonicalBacktestResult` were read from 0.63.0 source; later crate versions (0.67.0 docs exist) may differ. Whether starting balances and per-event account states can be recovered from it must be established in Part 4.
- The Nautilus and FIX order-state lists in §8.4 come from background knowledge, not sources fetched in this session.
- All performance figures are estimates until CP1.8 measures them. Thresholds are engineering goals, not facts.