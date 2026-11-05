# PART_03_INSTRUCTIONS.md — QuantLab-RS (qlrs) Part 3: Scale and Hardening

Status: instructions for an AI coding agent. Date of research: 2026-11-05.
Repository: Zakir032002/QuantLab-RS. Predecessor: Part 2 (dependency-aware reduction v1).
This document is self-contained. Do not assume any context not written here or in the repository.

---

## 0. Mission

Make `qlrs` robust against hostile input and run at 10^6–10^7 events, using Mode T (recorded trace) only. Every claim must be backed by committed raw evidence. Concretely:

1. Add coverage-guided fuzzing for the parser, fixed-point arithmetic, checker, reducer and bundle verifier, plus differential oracles.
2. Add explicit, typed resource limits, each mapped to a fixed status or error.
3. Replace "read the whole file and keep the source" with streaming ingest that has bounded memory, stores byte offsets, and re-reads witness lines with positioned reads.
4. Make the synthetic generator stream 10^7 events with bounded memory and recorded digests.
5. Commit measured baselines (instruction counts, wall time, peak RSS) and add regression gates.
6. Add Kani proofs for the arithmetic core, Miri runs and a stricter mutation ratchet.
7. Add supply-chain checks and a security review of the untrusted-input surface.

Exit gate: **Gate S** (section 19).

## 1. Scope and non-goals

In scope:
- `qlrs-io`, `qlrs-schema`, `qlrs-ledger`, `qlrs-check`;
- `qlrs-reduce` (performance and robustness only);
- `qlrs-bundle` (path safety and atomic writes);
- `qlrs-cli` (limit flags, statuses);
- `qlrs-testkit` (streaming generator), `qlrs-bench`;
- a new `fuzz/` workspace, CI workflows, docs and ADRs.

Non-goals. Do NOT do any of these. If a task seems to need one, stop and report.
- No real-engine import: no NautilusTrader or LEAN adapter (Parts 4, 5, 14).
- No Arrow or Parquet ingest or storage (Part 6).
- No UI or viewer (Parts 11–13).
- No Mode E (Part 16).
- No multi-currency.
- No PGO or BOLT (Part 17).
- No new invariants, unless fuzzing finds a real defect in an existing one. Even then, the fix needs human approval (rule R9).
- No reducer algorithm redesign. Constant-factor and setup-cost optimisations are allowed if outputs stay byte-identical.
- No change to the accepted `qlrs-trace/v0` language, except rejecting inputs that are already invalid under SPEC. Any newly rejected class of input needs an ADR and human approval.

## 2. Assumptions

- A1. Part 1 state at commit e348f56:
  - ten crates, edition 2024, Rust 1.99.1;
  - `#ǃ[forbid(unsafe_code)]` everywhere except an isolated test allocator;
  - 60 tests;
  - mutation floors: numeric/checker 347/387, reducer 78/117, fingerprints 11/11.
- A2. Part 2 may have renamed or added items: seeded binary reduction, algebraic evaluator, certificate, `qlrs-bundle/v1`, shapes S1–S9, catalogue D01–D14, ADRs from 0010. Use the names in `docs/HANDOFF_PART_02.md` and the code. Where this document uses a Part 1 name that has changed, apply the rebase table you write in CP3.0.
- A3. The release profile uses `panic = "abort"` with overflow checks on. Any panic on input is a process abort and counts as a defect.
- A4. The developer machine is an Apple Silicon Mac. CI uses GitHub-hosted runners.
- A5. Toolchain facts as of 2026-11-05: stable is 1.99.1, and 1.100.0 is scheduled for 2026-11-12 (releases.rs).  Re-verify at CP3.0.

## 3. Mandatory agent rules

- R1. **No unverified versions.** Before you add or use any crate or tool, record its exact version, release date and source (crates.io, docs.rs or GitHub release) in `docs/STACK.md`. If you cannot verify it, do not use it; report instead.
- R2. **No benchmark claims without committed raw outputs.** Every number in docs, commits or the handoff must point to a raw file under `evidence/part3/` (hyperfine JSON, gungraun output, `/usr/bin/time` output, libFuzzer logs). Each file records host, OS, CPU, toolchain, commit and command line.
- R3. **ADRs.** Every stable-interface change and every decision in section 5 gets an ADR, numbered from the next free number after Part 2's last ADR.
- R4. **Never weaken anything to pass.** This covers tests, tolerances, the protected catalogue, golden files, the golden manifest, the test-count ratchet, mutation floors, fingerprints, the float ban, the dependency-direction script and the determinism job. Ratchets may only go up.
- R5. **Two attempts, then stop.** If a task fails after two genuine attempts, stop. Write the blocker into `docs/HANDOFF_PART_03.md` under "Blockers" and report. Do not work around it silently.
- R6. **No floats in money paths.** The float ban still applies. Benchmark or report tooling may use floats only outside the library crates, and only for presentation.
- R7. **No fabricated results.** Never invent numbers, logs, versions, CPU-hours or pass states. "Not run" is an acceptable answer.
- R8. **One real commit per checkpoint**, with message prefix `CP3.N:`. No fake tags. Tag only when told to: `part3-gate-s` at Gate S, and only if every criterion passes.
- R9. **Verdict changes need human approval.** If a change alters a verdict (CONSISTENT/VIOLATION/INCONCLUSIVE/UNSUPPORTED), an exit code, a fingerprint or a golden output for an existing input, stop and request approval. Include a minimal input and a before/after diff.
- R10. **Fuzz crashes become regression tests before they are fixed.**
  1. Commit the minimised crash input to `tests/regressions/fuzz/<target>/<id>`, with a test that fails or is marked `#[ignore]` with a tracking ID.
  2. Fix the bug in a separate commit.
  3. Remove the `#[ignore]`.
- R11. **Fuzz time counts only with logs.** CPU-hours count only if a committed log shows libFuzzer's final stats (`-print_final_stats=1`), the command line, the commit and the duration.
- R12. **Determinism.** Verdicts, witnesses, bundles, fingerprints, digests and JSON reports must be byte-identical across thread counts, runs and macOS/Linux. Timing-based limits are the only allowed source of nondeterminism, and outputs affected by them must be labelled (section 12).
- R13. **No unsafe code** in the product crates. An exception needs an ADR plus human approval. None is planned in Part 3.
- R14. **No huge files in git.** Nothing over 5 MiB is committed, except files on the allowlist (section 11.6).

## 4. New requirement IDs (add to SPEC and TRACE)

| ID | Requirement |
|---|---|
| HARD-001 | No input bytes may cause a panic, abort, stack overflow, hang beyond the configured limits, or unbounded memory growth in `verify`, `reduce` or `bundle verify`. |
| HARD-002 | Every resource limit in section 7 is enforced and produces its specified status or error code, with a stable error identifier. |
| HARD-003 | Duplicate JSON keys, unknown keys, nesting deeper than the schema, oversized lines, oversized numeric literals and invalid UTF-8 are rejected as input errors, each with a distinct identifier. |
| HARD-004 | Arithmetic overflow anywhere in ledger or check paths is reported with the status SPEC already defines for it. It never wraps and never panics. |
| HARD-005 | Bundle verification never reads or writes outside the bundle root. It rejects symlinks, absolute paths and `..` components. |
| HARD-006 | Bundle and report writes are atomic: temp file in the same directory, fsync, rename. |
| SCALE-001 | `verify` on a 10^7-event generated trace completes on `ubuntu-24.04` and on Apple Silicon, with peak RSS and wall time recorded. |
| SCALE-002 | Ingest does not keep the full source bytes. Witness lines are re-read by offset and are byte-identical to the original lines. |
| SCALE-003 | Generating 10^7 events uses bounded memory (proposed: ≤ 64 MiB peak RSS) and produces a recorded digest. |
| SCALE-004 | Reduction at 10^6 and 10^7 events completes within recorded limits. The witness is lifecycle-valid and replays to the same violation fingerprint. |
| DET-001 | Outputs are byte-identical for `RAYON_NUM_THREADS` ∈ {1, 2, 3, 4, 8} and across macOS arm64 and Linux x86_64. |
| DET-002 | No output depends on `HashMap`/`HashSet` iteration order or on any `RandomState`. |
| PERF-001 | Instruction-count baselines are committed. CI fails on a regression above the section 13 threshold. |
| PERF-002 | The peak-RSS gate holds for the 1M and 10M presets. |
| FUZZ-001 | Each fuzz target in section 6 has a seed corpus, a dictionary where applicable, a smoke job and a nightly job. At Gate S each has ≥ 1 CPU-hour of logged fuzzing and zero unfixed crashes. |
| FV-001 | The Kani harnesses in section 14 pass at the documented bounds. |
| SUP-001 | cargo-deny passes in CI, release binaries are built with cargo-auditable, and the dependency policy is documented. |

## 5. Stable-interface changes and ADRs (number from the next free ADR)

| ADR topic | Decision to record |
|---|---|
| ADR-A: Streaming ingest and source non-retention | Two-phase ingest. Phase 1 is a chunked parse that stores `(offset: u64, len: u32)` per record. Witnesses are then fetched with positioned re-reads. Non-seekable inputs (stdin, pipes) are copied to a temp file, or rejected with `E-INPUT-NOT-SEEKABLE` when `--no-spool` is set. |
| ADR-B: mmap rejected | No `memmap2` and no `qlrs-mmap` crate in Part 3. Reasons: it needs unsafe, it can SIGBUS if the untrusted file is truncated, and positioned reads are enough. Revisit only in Part 6, with measurements. |
| ADR-C: Parser strategy and third-party unsafe policy | serde_json is the reference. A safe strict scanner is optional and adopted only if it passes the differential gate. SIMD JSON crates are bench-only. Third-party unsafe is allowed per crate, by ADR. |
| ADR-D: Resource limits and status mapping | The section 7 table becomes a stable interface (`Limits` struct, CLI flags, error identifiers). |
| ADR-E: Fuzz toolchain | cargo-fuzz in a separate workspace on a pinned nightly; corpus-replay test on stable; corpus and dictionary policy. |
| ADR-F: Allocator | The measured decision. The system allocator stays the default unless the section 10.6 evidence says otherwise. |
| ADR-G: Performance gate methodology | Instruction counts as the gate, wall time as a trend, the peak-RSS gate and its thresholds. |
| ADR-H: Generator format, RNG and presets | In-house SplitMix64/xoshiro256** with test vectors, streaming writer, presets, digests. |
| ADR-I: Supply-chain policy | cargo-deny config, cooldown (after 1.100), cargo-auditable, cargo-vet status. |
| ADR-J (only if done): Toolchain bump to 1.100.x | Only after 1.100.0 is actually released. Separate commit and re-baseline. |

Section 22 lists the stable interfaces Parts 4 and 5 will rely on. Do not change them after Gate S without an ADR.

## 6. Fuzz targets

General settings:
- `fuzz/` is its own Cargo workspace, not a member of the root workspace. `fuzz/rust-toolchain.toml` pins a dated nightly; verify and record the date at CP3.4.
- The tool is cargo-fuzz 0.13.2 (released 2026-06-09).  Re-verify at CP3.4.
- Every target must build with `--cfg fuzzing` without changing product semantics. Do not use `cfg(fuzzing)` to skip any check.
- Always pass `-rss_limit_mb=2048`, `-timeout=10` and `-print_final_stats=1`.
- Structured targets use `arbitrary` (verify the version) to build lifecycle-valid traces from the testkit shapes.
- Corpus replay:
  - `crates/qlrs-io/tests/fuzz_corpus_replay.rs`, plus one per other crate as needed, runs every file in `fuzz/corpus/<target>/` and `tests/regressions/fuzz/<target>/` through the target's oracle function.
  - It runs on stable Rust, in normal `cargo test`.
  - The oracle functions live in `qlrs-testkit::oracles`, so `fuzz/` and the tests share the same logic.

| Name | Input | Oracle(s) | Sanitizer (CI nightly / long) | Smoke (PR) | Nightly | Gate S long | Seed corpus | Dictionary |
|---|---|---|---|---|---|---|---|---|
| F01 `io_line` | raw bytes as one line, `-max_len=4096` | No panic. Result is `Ok(record)` or `Err` with a stable identifier. If `Ok`, re-encoding the record canonically and re-parsing it gives an equal record | ASan / none | 60 s | 20 min | ≥ 3600 CPU-s | One line per event kind from every generator profile and shape; every golden-file line; every limit-edge case from section 7 | `fuzz/dict/qlrs_trace_v0.dict` |
| F02 `io_trace` | raw multi-line bytes, `-max_len=1048576`, small `Limits` | No panic, no hang, memory under the rss limit. Result is a valid `TraceStore` or a typed error whose line number is ≤ the line count | ASan / none | 60 s | 20 min | ≥ 3600 CPU-s | Small traces (≤ 64 KiB) from each shape S1–S9, plus the D01–D14 witnesses | same |
| F03 `io_chunk_diff` | bytes, plus an arbitrary chunk size (1 B–64 KiB) and thread count (1–8) | Chunked parallel ingest and sequential ingest give identical `TraceStore` digests, interner IDs and first errors | none / none | 60 s | 20 min | ≥ 3600 CPU-s | as F02 | same |
| F04 `io_parser_diff` (only if the CP3.8 scanner exists) | raw multi-line bytes | Scanner and serde_json reference agree on accept/reject, records, error class and line | ASan / none | 60 s | 30 min | ≥ 3600 CPU-s | as F02, plus all F01 seeds | same |
| F05 `num_ops` | structured: two `Fixed` values (raw i128, scale 0–18) and an op from {add, sub, mul, div, rescale, cmp, neg} with rounding mode | Result equals a big-integer oracle exactly, or both report overflow/invalid. Never panics. Scale stays ≤ 18 | none / none | 60 s | 20 min | ≥ 3600 CPU-s | Boundary values: 0, ±1, i128::MIN/MAX, 10^18, 10^18−1, values near the U256 fallback | n/a |
| F06 `num_decimal` | raw bytes as a decimal literal | `parse(format(x)) == x`. Literals over the length limit are rejected. Accepted literals match the oracle parse | none / none | 60 s | 20 min | ≥ 3600 CPU-s | `0`, `-0`, `1e5` (must be rejected if SPEC rejects exponents), 39- and 40-digit strings, leading zeros, `+1`, `.5`, `5.` | digits, `-`, `.`, `e`, `E`, `+` |
| F07 `check_clean` | structured lifecycle-valid trace with no injected defect, ≤ 2,000 events | Never VIOLATION on clean input. CONSISTENT unless SPEC requires INCONCLUSIVE (e.g. a lifecycle still open at the end). Same result at thread counts 1 and 4 | none / none | 60 s | 20 min | ≥ 3600 CPU-s | Generated from S1–S9 at small sizes | n/a |
| F08 `check_injected` | structured clean trace plus one injected defect of a D01–D14 kind | (a) Verdict is VIOLATION with the expected invariant class. (b) First-violation seq ≥ the injection seq. (c) Prefix property: if the full trace is CONSISTENT, every prefix is CONSISTENT or INCONCLUSIVE, never VIOLATION. If a prefix is VIOLATION, the full trace is VIOLATION with a first seq ≤ the prefix's | none / none | 60 s | 20 min | ≥ 3600 CPU-s | D01–D14 witnesses | n/a |
| F09 `check_dense_vs_selected` | structured trace plus a random subset of units | `check_selected(all units)` ≡ `check`. Per-unit results agree with `check` restricted to that unit. `checkpoint_at_claim` followed by replay equals a full replay at that seq | none / none | 60 s | 20 min | ≥ 3600 CPU-s | as F07/F08 | n/a |
| F10 `reduce_sound` | structured violating trace (the F08 generator); `--max-calls` limit only, no time limit | The witness replays to VIOLATION with the same fingerprint class, is lifecycle-valid, is no larger than the original, and its lines are byte-identical slices of the original. Reducing the same input twice gives byte-identical output. If Part 2 has a certificate, it verifies | none / none | 60 s | 30 min | ≥ 3600 CPU-s | D01–D14 | n/a |
| F11 `eval_vs_checker` (only if the Part 2 algebraic evaluator exists) | structured candidate subsets | The fast evaluator's verdict equals the full checker's verdict on the materialised subset | none / none | 60 s | 20 min | ≥ 3600 CPU-s | Part 2 test inputs | n/a |
| F12 `bundle_verify` | raw bytes read as a bundle archive or directory manifest, unpacked under a temp root | No panic. No filesystem access outside the root (assert with a path-checking wrapper). Rejects symlinks, absolute paths, `..`, duplicate entries, and entries over the size or count limits. Tampering is detected | ASan / none | 60 s | 20 min | ≥ 3600 CPU-s | Valid v0 bundles (and v1 if present) from tests, plus truncated copies | bundle manifest keys |

Dictionary `fuzz/dict/qlrs_trace_v0.dict`:
- Generate it with `scripts/gen_fuzz_dict.sh` from the schema key list in SPEC, so it never drifts from the schema.
- Add structural tokens: `"{"`, `"}"`, `"["`, `"]"`, `":"`, `","`, `"\""`, `"\\u0000"`, `"\\ud800"`, `"\n"`, `"\r\n"`, `"true"`, `"false"`, `"null"`, `"-"`, `"."`, `"e"`, `"E"`.
- Add boundary numbers: `"9223372036854775807"`, `"170141183460469231731687303715884105727"`, `"340282366920938463463374607431768211455"`, `"1000000000000000000"`.
- Add the schema version string `"qlrs-trace/v0"` and every enum value string in SPEC.

Command lines (Linux, ASan nightly):
```
cd fuzz
cargo fuzz build --release io_trace
cargo fuzz run io_trace -- -max_total_time=1200 -rss_limit_mb=2048 -timeout=10 -max_len=1048576 -dict=dict/qlrs_trace_v0.dict -print_final_stats=1 2>&1 | tee ../evidence/part3/fuzz/io_trace/$(date -u +%Y%m%dT%H%M%SZ)-$(git rev-parse --short HEAD).log
```
Long campaign, ≥ 1 CPU-hour (4 workers × 15 min on `ubuntu-24.04`, or 3 workers × 20 min on the M1):
```
cargo fuzz run -s none io_trace -- -jobs=4 -workers=4 -max_total_time=900 -rss_limit_mb=2048 -timeout=10 -dict=dict/qlrs_trace_v0.dict -print_final_stats=1
```
- Record CPU-seconds as the sum over workers, taken from the logs.
- Use whichever of `-jobs/-workers` or `-fork` the pinned cargo-fuzz/libFuzzer documents. Verify at CP3.4 and record it in ADR-E.

Corpus management:
- Run `cargo fuzz cmin <target>` before every corpus commit.
- Committed corpora are capped at ≤ 2,000 files and ≤ 20 MiB per target (proposed).
- Crashes: minimise with `cargo fuzz tmin <target> <artifact>`, then follow R10.
- Coverage: for the parser and num targets, run `cargo fuzz coverage <target>` and then `llvm-cov report` (or `cargo-llvm-cov`, version verified). Commit the summary text to `evidence/part3/coverage/`, not the raw profiles.
- Local macOS: add `ASAN_OPTIONS=detect_leaks=0`, because rust-lang/rust#121624 reports false leaks on aarch64-apple-darwin. 

## 7. Resource limits

Add a `Limits` struct in `qlrs-io`, re-exported by the CLI, and the CLI flags below.
- Defaults are proposed; record the final values in ADR-D.
- Every limit gets unit tests at limit−1, at the limit and at limit+1.

| Key / flag | Default (proposed) | Hard ceiling | Applies to | On exceed: status / error id | Exit |
|---|---|---|---|---|---|
| `max_line_bytes` / `--max-line-bytes` | 1 MiB | 16 MiB | each line, before parsing | Input error `E-LIMIT-LINE-BYTES` (with line no.) | 1 |
| `max_json_depth` | 4, or schema depth + 1 if smaller | 8 | each line | Input error `E-LIMIT-DEPTH`. Keep serde_json's 128 limit on; never enable `unbounded_depth`  | 1 |
| `max_numeric_literal_len` | 40 chars (i128 is ≤ 39 digits plus sign) | 64 | each numeric or decimal string | Input error `E-LIMIT-NUMBER-LEN` | 1 |
| `max_decimal_scale` | 18 (existing) | 18 | decimals | Input error `E-SCALE` (existing behaviour; confirm) | 1 |
| duplicate key | — | — | every JSON object | Input error `E-DUPLICATE-KEY`, including through any flatten or optional path (serde#2416)  | 1 |
| unknown key | — | — | every object | Input error `E-UNKNOWN-KEY` (confirm against current strict mode) | 1 |
| invalid UTF-8 / lone surrogate escapes | — | — | each line | Input error `E-UTF8` | 1 |
| `max_string_bytes` (ids, symbols) | 256 B | 4 KiB | each string field | Input error `E-LIMIT-STRING` | 1 |
| `max_interned` / `--max-interned` | 16,777,216 distinct strings | u32::MAX − 1 | interner | UNSUPPORTED `U-LIMIT-INTERNED` | 4 |
| `max_events` / `--max-events` | 200,000,000 | u32::MAX − 1 (4,294,967,294) | event count | UNSUPPORTED `U-LIMIT-EVENTS` | 4 |
| CSR offsets, unit counts, edge counts | — | u32::MAX − 1 | `qlrs-schema`, `qlrs-reduce` | UNSUPPORTED `U-LIMIT-INDEX`. Every `usize`→`u32` conversion uses checked `try_from` | 4 |
| `max_input_bytes` / `--max-input-bytes` | 64 GiB | u64 | file size (`metadata().len()`) | Input error `E-LIMIT-INPUT-BYTES` | 1 |
| `memory_budget_bytes` / `--memory-budget` | unset (not enforced) | — | estimated from the pre-scan (line count × bytes/event model) before columns are allocated | INCONCLUSIVE `I-BUDGET-MEMORY` (labelled resource-bound) | 3 |
| `--time-limit-ms` (existing) | existing | — | reduce (and verify if present) | Existing status. Output labelled `nondeterministic: time-limit` (section 12) | existing |
| `--max-calls` (existing) | existing | — | reduce | Existing status; deterministic | existing |
| bundle `max_entries` | 10,000 | — | bundle verify | Input error `E-BUNDLE-ENTRIES` | 1 |
| bundle `max_entry_bytes` | 1 GiB | — | bundle verify | Input error `E-BUNDLE-ENTRY-BYTES` | 1 |
| bundle path safety | — | — | bundle verify | Input error `E-BUNDLE-PATH` (absolute, `..`, symlink, non-UTF-8, duplicate) | 1 |
| non-seekable input with `--no-spool` | — | — | ingest | Input error `E-INPUT-NOT-SEEKABLE` | 1 |
| arithmetic overflow | — | — | ledger and check | Whatever status SPEC defines today (confirm at CP3.0; do not change without approval) | existing |

Mapping rule:
- Malformed input → input error (exit 1).
- Well-formed input beyond the model's representational capacity → UNSUPPORTED (exit 4).
- A user-set budget being hit → INCONCLUSIVE (exit 3).
- If the current code maps any of these differently, changing it is a verdict change under R9: stop and request approval.

Error identifiers are stable strings. They appear in JSON output and on the first line of stderr, and are listed in `docs/ERRORS.md`.

## 8. Streaming I/O and memory design

### 8.1 Alternatives evaluated

| Option | Safety | Memory | Throughput | Decision |
|---|---|---|---|---|
| Current: `fs::read` the whole file, keep the source and line ranges | safe | source (~179 B/event) plus columns | good | Replace |
| `memmap2` in an isolated `qlrs-mmap` crate with an ADR | needs unsafe; SIGBUS if the untrusted file is truncated during the run | page cache, which RSS does not count the same way | good | **Rejected for Part 3** (ADR-B) |
| Read fixed chunks with `BufReader`/`read`, parse, drop the buffer; store `(u64 offset, u32 len)` per record; re-read witness lines with `FileExt::read_at` | safe std (`std::os::unix::fs::FileExt`, macOS and Linux) | columns + offsets (12 B/event) + O(chunk × in-flight) | expected to match or beat the current path (measure) | **Adopted** (ADR-A) |
| Single streaming pass with no random access | safe | minimal | best | Not enough: the reducer and witness writer need random access |

Windows is not a target. If it is ever added, the `FileExt::seek_read` equivalent goes behind a small platform module. Do not add Windows CI in Part 3.

### 8.2 Design

1. **Phase 0, open.**
   - `File::open`, then `metadata()`.
   - Reject anything that is not a regular file, unless spooling. Spooling copies stdin and pipes to a temp file in `$TMPDIR`, enforcing `max_input_bytes` during the copy.
   - Apply `max_input_bytes`.
2. **Phase 1, chunked parse.**
   - Read fixed-size chunks: default 8 MiB, flag `--io-chunk-bytes`. Measure 1, 4, 8 and 16 MiB.
   - Split each chunk at its last `\n` and carry the partial tail into the next chunk.
   - A line longer than `max_line_bytes` fails before it is ever buffered whole.
   - Batches of complete lines go to rayon, with a bounded number of chunks in flight (default `2 × threads`). Memory stays O(chunk × in-flight).
   - Each batch produces column fragments, a local interner (first-occurrence order within the batch), an offsets fragment, and its first error if any.
3. **Phase 2, deterministic merge.**
   - Merge fragments strictly in chunk order.
   - Assign global interner IDs by walking the local interners in chunk order, then in local first-occurrence order. This exactly reproduces the IDs a sequential parse would give.
   - Remap local IDs to global IDs in one pass over each fragment.
   - The reported first error is the one with the smallest line number across all chunks, which is what sequential parsing would report. F03 checks this.
4. **Columns.**
   - Keep the per-kind SoA columns.
   - Use `Vec::with_capacity` from the Phase 0 estimate: an upper bound of file bytes ÷ minimum bytes per line, capped by `max_events`. Call `shrink_to_fit` once at the end.
   - Measure whether over-allocating capacity beats letting the vectors grow.
5. **No source retention.**
   - Witness and bundle writers fetch lines by `(offset, len)` with `read_at`.
   - Each fetched line must end at `\n` or at EOF, and must match a per-line or per-chunk digest recorded in Phase 1.
   - If the file changed between phases, the digest mismatch is input error `E-INPUT-CHANGED`.
6. **Offsets.** Either `Vec<u64>` starts plus `Vec<u32>` lengths (12 B/event), or u32 start deltas per 4 GiB segment (8 B/event). Choose by measurement and record the choice in ADR-A.
7. **Peak memory accounting.**
   - Add `TraceStore::heap_bytes_estimate()`: the sum of `capacity × size_of` over all columns. Print it in the `--stats` JSON.
   - Measure process peak RSS externally (section 13).
   - No counting global allocator in product code, because it needs unsafe. The existing isolated test allocator may be used in tests only.

### 8.3 Interner

- Use the sequential-merge design from 8.2.3.
- A sharded concurrent interner is rejected: its ID order would depend on scheduling, and it needs locking.
- Use a hash map with a fixed-seed hasher. Choose the hasher by measurement, and record the crate and version per R1.
- No `RandomState` anywhere in the I/O builder (DET-002).
- Store strings in one arena `String`, with `Vec<u32>` offsets.
- Enforce `max_string_bytes` and `max_interned`.

## 9. Parser strategy

1. **Reference.** Keep the serde_json borrowed-`Cow` path as the reference parser, selectable with `--parser reference`.
2. **Experiment (CP3.8): `qlrs-io::scan`.** A safe, hand-written strict line scanner for our schema:
   - matches the fixed key set from SPEC byte by byte, along with the values;
   - validates number literals against the section 7 limits;
   - does not allocate for valid lines, and borrows strings;
   - rejects everything the reference rejects: duplicate keys, unknown keys, depth, escapes, invalid UTF-8.
3. **Adoption criteria (all required):**
   - (a) ≥ 1.3× fewer instructions per event than the reference on the 1M preset (proposed);
   - (b) at least 1 CPU-hour of F04 differential fuzzing with zero divergences;
   - (c) corpus replay and all goldens identical;
   - (d) mutation score on `scan` at or above the `qlrs-io` floor.

   If the scanner is adopted, the reference stays in the binary for `--parser reference` and in the CI differential tests permanently.
4. **SIMD crates.** sonic-rs 0.5.10 and simd-json 0.19.0 as of research;  re-verify.
   - Measure them only in `qlrs-bench`, behind a bench-only feature. Report instructions/event and MB/s.
   - For sonic-rs, pin sonic-number ≥ 0.1.3 (cloudwego/sonic-rs issue #246). 
   - Keep them off the product path in Part 3:
     - they rely heavily on internal unsafe code;
     - sonic-rs needs `-C target-cpu=native` for its SIMD paths and a special feature under sanitizers; 
     - tokenisation may not be our bottleneck.
   - The crate-level `forbid(unsafe_code)` does not cover dependencies, so ADR-C governs third-party unsafe.
5. **Numbers.** Never parse money through floats. Decimal literals go straight to `Fixed`, after a digit-count check.

## 10. Performance techniques and where each applies

For each change:
1. Measure before, and commit the result.
2. Make the change.
3. Measure after, and commit the result.
4. Confirm goldens and fingerprints are identical.
5. Record the delta in `docs/PERF_LOG.md`.

Revert any change that does not improve instructions or RSS beyond noise.

| # | Technique | Where | Expected effect (estimate, verify) |
|---|---|---|---|
| 10.1 | Replace `Box<dyn Iterator>` in the check loop with a concrete enum iterator or a generic, monomorphised loop | `qlrs-check` | Removes a virtual call per event and allows inlining |
| 10.2 | Compute the `quantity_limit` column once, at prepare | `qlrs-ledger`/`qlrs-check` | Removes recomputation on every order event |
| 10.3 | Fused reducer setup: build dependency edges, unit indices and lifecycle spans in one scan | `qlrs-reduce` | Fewer passes over 10^7 events. Outputs must stay byte-identical |
| 10.4 | Fixed-seed fast hasher, or index-based maps where keys are dense u32 | `qlrs-io` builder | Cheaper hashing; deterministic |
| 10.5 | Column compaction: smallest integer type per column, `u8` kind tags, bitsets for booleans | `qlrs-schema` | Moves toward 100 B/event for the whole process |
| 10.6 | Allocator A/B: system vs mimalloc 0.1.52 vs tikv-jemallocator 0.7.0 (re-verify versions), behind a `bench-alloc` feature in `qlrs-cli` only | `qlrs-cli` | Measure peak RSS, instructions and wall time on 1M and 10M, on both OSes. Adopt only if better on both, and record in ADR-F. Both are C dependencies |
| 10.7 | Keep overflow checks on in release. Money paths use explicit `checked_*` (already done). Index arithmetic may use `wrapping_*` (never unchecked ops) only where a proof or test shows it cannot overflow | all | Measure the cost of overflow checks once with a build that disables them, for information only. Never ship that build |
| 10.8 | Branch-light invariant evaluation: compute violation flags arithmetically, accumulate them, and branch once per batch to find the first violation | `qlrs-check` | Faster on clean traces. F08 must confirm first-violation semantics are unchanged |
| 10.9 | `#[inline]` on small hot functions used across crates; `#[cold]` on error constructors and the U256 fallback | `qlrs-num`, `qlrs-check` | Guided by samply/perf profiles, not PGO |
| 10.10 | Keep rayon. Tune chunk sizes by measurement, avoid `par_iter` over tiny slices, bound in-flight batches (8.2) | `qlrs-io` | Merge stays deterministic |
| 10.11 | `[profile.release]` stays fat LTO, `codegen-units=1`, `panic=abort`, overflow checks on. Add `[profile.profiling]` inheriting release with `debug = "line-tables-only"` for samply, perf and Instruments | workspace | No behaviour change |

Profiling tools (verify versions and record them in STACK.md before use):
- **Linux:** `perf`, samply, cargo-flamegraph, gungraun/Valgrind, heaptrack or dhat-rs (dev-only, test-gated), and bytehound only if it is still maintained (verify).
- **macOS:** samply, Instruments (`xcrun xctrace`), `/usr/bin/time -l`, hyperfine.
- Valgrind is not available on macOS arm64, so the gungraun gates run on Linux only.

## 11. Generator scale specification

1. **RNG.**
   - In-house SplitMix64 for seeding and xoshiro256** for the stream, in `qlrs-testkit::rng`.
   - Commit test vectors: the first 16 outputs for seeds 0, 1 and 0xDEADBEEF. Check them in CI on macOS and Linux.
   - No `rand` dependency in the generator path, so the stream cannot drift between rand versions.
2. **Streaming writer.**
   - Generate events in seq order and write them through a 1 MiB `BufWriter` to a temp file, then fsync and rename (atomic).
   - Memory is bounded by live state (open orders, positions, cash), not by event count. Proposed bound: ≤ 64 MiB peak RSS at 10^7 events (SCALE-003).
   - Keep lifecycle validity incrementally as events are generated.
3. **Digest.**
   - Compute a digest over the output bytes while writing, with the same algorithm the fingerprints use. If there is none, choose one in ADR-H and verify the crate per R1.
   - Write `<file>.meta.json` with the generator version, profile/shape, seed, event count, byte count, digest and `qlrs` commit.
   - Commit only `evidence/part3/gen/digests.json`, which is small. Never commit generated traces.
4. **Presets** (`qlrs-testkit` binary `qlrs-gen`, or `cargo xtask gen`):

| Preset | Events | Approx. file size (estimate) | Where it runs |
|---|---|---|---|
| `s1m` | 1,000,000 | ~180–240 MB | PR perf job (instruction counts on a 100k subset); nightly |
| `s10m` | 10,000,000 | ~1.8–2.4 GB | Nightly on Linux; weekly on macOS arm64; local Mac |
| `s100m` | 100,000,000 | ~18–24 GB | **Local only**, behind `--allow-huge`; does not fit a 14 GB runner SSD |

5. **Profiles.** simple, interleaved, long-lived and sparse, plus the Part 2 shapes S1–S9 where they scale. Injected-defect variants put a D-catalogue defect at a seeded position (early, middle, or in the last 1%) for reduction-at-scale tests.
6. **Keeping huge files out of git.**
   - `.gitignore` covers `gen/`, `*.jsonl` outside `tests/`, `fuzz/artifacts/` and `fuzz/coverage/`.
   - `scripts/check_file_sizes.sh` runs in CI and rejects any tracked file over 5 MiB, except those in `scripts/large_file_allowlist.txt` (empty by default).

## 12. Robustness and determinism

- **Thread counts.** The CI determinism job runs `verify`, `reduce` and `bundle` on fixed inputs (the golden set, plus `s1m` checked by digest) with `RAYON_NUM_THREADS` ∈ {1, 2, 3, 4, 8}. It compares the SHA-256 of every output file.
- **Cross-OS.** The same inputs run on `ubuntu-24.04`, `ubuntu-24.04-arm` and `macos-15` (arm64). A final job compares the artifacts.
- **Hash iteration.**
  - No output-producing function may iterate a `HashMap` or `HashSet`. Enforce this with clippy's `iter_over_hash_type` if the pinned clippy has it (verify), otherwise with a grep script run at test time.
  - `RandomState` (std or foldhash) is allowed only in lookup-only maps, marked `// DET: lookup-only`.
- **Floats.** The existing float ban stays. Extend its script to the new crates and the `fuzz/` oracles.
- **Locale and paths.**
  - Output never depends on locale.
  - Outputs never contain platform path separators; bundle-relative paths use `/`.
  - Hashed outputs never use `Path::display`. They use the bundle-relative UTF-8 path, or reject non-UTF-8 paths. Paths shown to users are converted lossily but deterministically.
  - `\n` is the canonical line ending. A `\r\n` input is accepted or rejected exactly as SPEC says (confirm, and test both cases).
- **Timing-based limits.** Any output produced under `--time-limit-ms` carries `"determinism": "time-limited"` in its JSON report and bundle manifest. Determinism gates use only `--max-calls` runs.
- **Atomic writes.**
  1. Write each output to `<name>.tmp.<pid>.<n>` in the target directory.
  2. `sync_all`, then `rename`.
  3. On Unix, fsync the directory. Verify that `File::open(dir)?.sync_all()` works on macOS; if it does not, document it.

  On error, remove the temp file; never leave a partial output under the final name. Test this by injecting an I/O failure through a writer wrapper.
- **Bundle verification paths.**
  - Check every entry path with `Path::components()`: only `Normal` components; no `RootDir`, `Prefix` or `ParentDir`.
  - Use `symlink_metadata` and reject symlinks and non-regular files.
  - Open files relative to the canonicalised root, and re-check that each canonical path stays under the root.
  - Apply the size and count limits from section 7.
- **Security review.** Write `docs/SECURITY_REVIEW_PART3.md` covering:
  - the threat model: a local tool, untrusted trace and bundle files, no network;
  - the attack surface: parser, decimal parsing, interner, bundle unpacking, temp-file spooling, output paths;
  - each mitigation, with its test ID;
  - residual risks.

## 13. Baselines and regression methodology

Instruments:
- **Gate (PR): gungraun instruction counts** (Valgrind, `ubuntu-24.04` only).
  - Benches: parse 100k, check 100k, replay 100k, reduce D03 100k, generator 100k.
  - Measure base and head in the same job on the same runner. Callgrind counts can vary across runner CPUs because glibc picks implementations per CPU (IFUNC), so never compare against numbers from another machine. 
  - **Fail if any bench regresses by more than 3% (proposed).** An improvement above 3% must update the committed baseline in the same PR, with raw output.
- **Trend (nightly): hyperfine wall time** on `s1m` and `s10m` (`--warmup 2 --runs 10 --export-json`), on Linux and macOS.
  - Store the results as artifacts (90-day retention, the public-repo maximum)  and summarise them in `evidence/part3/trend/`.
  - Never a gate.
- **Peak RSS gate (nightly).** Measure with `/usr/bin/time -v` on Linux ("Maximum resident set size (kbytes)") and `/usr/bin/time -l` on macOS (bytes). Proposed thresholds:
  - `verify s1m`: ≤ 180 MiB.
  - `verify s10m`: ≤ 140 bytes/event + 64 MiB ≈ 1.4 GiB. Stretch target: ≤ 100 bytes/event + 64 MiB.
  - `reduce s10m` (injected defect): ≤ 2× the verify peak.
  - `qlrs-gen s10m`: ≤ 64 MiB.
- **Run metadata.** Every recorded run captures the commit, toolchain (`rustc -Vv`), OS, CPU model, core count, RAM, runner image version (`ImageOS`/`ImageVersion`), command line, input digest and raw output.

CodSpeed is optional. Its simulation mode is equivalent to our gungraun gate. Its macro runners reportedly need an organisation account, and this repository belongs to a user.  Do not depend on CodSpeed for Gate S.

Apple Silicon local measurement (`scripts/mac_bench.sh`, for Zakir's Mac):
1. **Prepare the machine.** Plug in power, turn off Low Power Mode, close other apps and wait until it is idle (`top -l 1 | head`). Record the chip, core counts and memory: `sysctl -n machdep.cpu.brand_string hw.perflevel0.physicalcpu hw.perflevel1.physicalcpu hw.memsize`.
2. **Build.** `cargo build --profile release --locked`.
3. **Generate.** `target/release/qlrs-gen --preset s10m --seed 1 --out /tmp/qlrs/s10m.jsonl`, then check the digest against `evidence/part3/gen/digests.json`.
4. **Wall time.** `hyperfine --warmup 2 --runs 10 --export-json evidence/part3/mac/verify_s10m.json 'target/release/qlrs verify /tmp/qlrs/s10m.jsonl'`.
5. **Peak RSS.** `/usr/bin/time -l target/release/qlrs verify /tmp/qlrs/s10m.jsonl 2> evidence/part3/mac/verify_s10m.time.txt`.
6. **Threads.** Repeat step 5 with `RAYON_NUM_THREADS` set to 1, to the number of P-cores, and to all cores. Compare the output digests.
7. **Profile.** `samply record target/profiling/qlrs verify /tmp/qlrs/s1m.jsonl`, or `xcrun xctrace record --template 'Time Profiler' --launch -- target/profiling/qlrs verify /tmp/qlrs/s1m.jsonl`. Commit only screenshots or summaries, never the traces.
8. **Optional.** Run `s100m` with `--allow-huge` if at least 30 GB of disk is free.

## 14. Formal verification scope

**Kani.** 0.68.0 was released on 2026-09-16; re-verify. Kani pins its own nightly.
- Harnesses live under `#[cfg(kani)]` in `qlrs-num` and `qlrs-schema`.
- They run in a weekly CI job and on manual dispatch, with a timeout per harness and recorded solver times.

| Harness | Property | Bound |
|---|---|---|
| K01 `fixed_checked_add` | Equals the mathematical sum when it fits; otherwise reports overflow; never panics | All i128 raw values, scales 0–18 (symbolic) |
| K02 `fixed_checked_mul_rescale` | Matches the specified rounding mode; no silent truncation | Symbolic raw values; if needed for tractability, bound to \|raw\| < 2^80 and record the bound |
| K03 `fixed_rescale_roundtrip` | Rescaling up and then down is the identity when the rescale is exact | Scales 0–18 |
| K04 `fixed_cmp_total_order` | Comparison across scales is a total order consistent with the mathematical value | Symbolic |
| K05 `decimal_parse_bounds` | Never accepts a literal longer than `max_numeric_literal_len`; never panics | Strings ≤ 8 bytes |
| K06 `alivemask_ops` | set, clear, test, count and iteration agree with a reference `[bool; N]` | N ≤ 130 bits (spans 2 words) |
| K07 `prefix_sum_lemma` | The prefix sum of a column equals the sequential fold at every index, with no overflow when inputs are within the declared bounds | Length ≤ 8 |

**Miri** (nightly component; verify).
- Run `cargo +nightly miri test` on the isolated test-allocator crate and on `qlrs-num` and `qlrs-schema`.
- Exclude tests that are too slow under Miri with a cfg gate; do not delete them.
- Weekly job.

**Not used in Part 3:**
- Verus, Creusot and Prusti: their 2026 status was not verified, and their cost is not justified for this scope.
- Loom and shuttle: we have no custom synchronisation primitives. Determinism is covered by the section 12 tests and F03.
- bolero: keep any existing proptest tests and do not add bolero. ADR-E records why.

## 15. Mutation ratchet plan

- **Tool.** cargo-mutants 27.1.0 (re-verify). Use `test_tool = "nextest"` if nextest is already in use (verify the cargo-nextest version); otherwise use cargo test.
- **PR job.** `cargo mutants --in-diff <(git diff origin/main...HEAD) --test-tool nextest --timeout-multiplier 3`; verify the flag spellings against the installed version. The job fails if new code has a missed mutant that is not justified in `mutants-allowlist.md`.
- **Nightly job.** A full run with `--shard k/4 --sharding round-robin` (verify the syntax) across 4 matrix jobs on `ubuntu-24.04`, merging the `outcomes.json` files.
- **Ratchet.**
  - Floors are per area: numeric/checker, reducer and fingerprints, plus the new areas io-limits, io-stream, scan (if adopted), generator and bundle-paths.
  - Each floor is a caught/viable pair, stored in `mutants-floors.toml`.
  - Floors only go up. After each checkpoint that adds tests, raise them to the new measured values.
  - Proposed Gate S target: every area ≥ 90% caught of viable mutants, except where the allowlist explains equivalent mutants.
- **Timeouts** count as caught only if they reproduce twice, and are recorded separately.

## 16. CI plan

| Workflow | Trigger | Runner(s) | Jobs | Expected duration (estimate) |
|---|---|---|---|---|
| `ci.yml` (existing, extended) | PR, push | ubuntu-24.04, macos-15 | build, test, ratchets, float ban, dependency direction, file-size check, corpus replay (stable) | ≤ 20 min |
| `fuzz-smoke.yml` | PR | ubuntu-24.04 | Build all targets on the pinned nightly; run each for 60 s with `-s none`; fail on any crash | ≈ 15–20 min for 12 targets |
| `perf-gate.yml` | PR | ubuntu-24.04 | gungraun base vs head in one job; 3% gate (proposed) | ≈ 15–25 min |
| `mutants-diff.yml` | PR | ubuntu-24.04 | `--in-diff` | ≤ 30 min |
| `nightly.yml` | daily schedule, manual | ubuntu-24.04 (matrix), ubuntu-24.04-arm, macos-15 | ASan fuzzing, 20–30 min per target (matrix; each job ≤ 6 h); `s1m`/`s10m` generate, verify and reduce with RSS and time; determinism thread matrix; mutants shards; hyperfine trend | ≈ 2–4 h per matrix leg, under the 6 h job limit |
| `weekly.yml` | weekly schedule, manual | ubuntu-24.04, macos-15 | Kani, Miri, coverage reports, `s10m` on macOS arm64, full cargo-deny, cargo-vet (non-blocking) | ≤ 4 h |
| `fuzz-long.yml` | manual dispatch | ubuntu-24.04 matrix, one target per job | 4 workers × 15 min = 1 CPU-hour per target; uploads logs, and the agent commits trimmed copies | ≈ 20 min per job |

ARM fallback rule. If a `macos-15` (arm64) job is still queued after 30 minutes, or fails with a runner-capacity error, the workflow:
1. re-queues it once;
2. if that also fails, runs it on `macos-15-intel`, labelled `fallback-intel`;
3. never counts an Intel run toward an Apple Silicon criterion.

Gate S criteria that need Apple Silicon can instead be met by a run on Zakir's Mac using `scripts/mac_bench.sh`, with the outputs committed. `macos-15-intel` is the last Intel image; plan for it to disappear in 2027.

Artifacts are kept 90 days, the public-repo maximum. Anything Gate S depends on must also be committed, trimmed, under `evidence/part3/`, not left only in artifacts.

Toolchain pinning:
- The root `rust-toolchain.toml` stays at 1.99.1 until ADR-J.
- `fuzz/rust-toolchain.toml` pins a dated nightly.
- Kani uses its own pinned toolchain.

## 17. Supply-chain tasks

1. **`deny.toml`**, checked by cargo-deny 0.20.2 (re-verify) as a blocking CI job:
   - advisories: deny vulnerabilities, yanked and unsound crates; deny unmaintained crates at least among direct dependencies;
   - licenses: explicit allowlist;
   - bans: deny wildcards, warn on duplicates;
   - sources: crates.io only.
2. **cargo-audit 0.22.2.** Optional local tool; do not add a redundant CI job.
3. **cargo-vet.**
   - Run `cargo vet init` with exemptions, as a non-blocking weekly job.
   - Do not import third-party audit sets without recording their source.
   - Confirm the version on crates.io first. At research time crates.io and docs.rs listed 0.10.2 (2026-01-13), while the GitHub release 0.10.3 is dated 2026-10-22.
   - Pin the exact version: zcash/zallet PR #918 reports that 0.10.3 adds a store check requiring versioned policy entries, which breaks CI that installs via `~0.10`.
4. **Release binaries.**
   - Build them with cargo-auditable 0.7.7 (re-verify).
   - Add GitHub artifact attestations for release artifacts (GitHub docs: "Artifact attestations").
   - SBOM: choose and verify a tool at CP3.13 (e.g. a CycloneDX cargo plugin). If none can be verified, record "SBOM not produced" in the handoff.
5. **Cooldown.** After the toolchain moves to Rust ≥ 1.100.0 (ADR-J), add to `.cargo/config.toml`:
```
   [registry]
   global-min-publish-age = "7 days"
```
   - It is opt-in and defaults to zero (per coverage of the 1.100 beta notes). Check the key against the released Cargo book before committing.
   - It does not apply to `cargo install`, so pin CI tool installs by version and checksum. 
6. **GitHub Actions.** Pin every action by commit SHA. Default to `permissions: contents: read`.
7. **`fuzz/Cargo.lock`.** Either commit it or generate it from the root lock. Choose one and document it in ADR-E.

## 18. Checkpoints

Each checkpoint ends with:
- every verification command passing;
- one commit with the prefix `CP3.N:`;
- an updated progress table in `docs/HANDOFF_PART_03.md`;
- its evidence committed.

If any pass criterion is false, apply R5.

### CP3.0 — Verify Part 2 and inventory
- **Objective:** confirm that Part 2's gates passed and that the environment and versions are current.
- **Tasks:**
  1. Read `docs/HANDOFF_PART_02.md`. List every Part 2 gate and the evidence it claims.
  2. Re-run:
     - `cargo test --workspace --locked`;
     - the ratchet scripts, the golden manifest check, the float ban and the dependency-direction script;
     - the determinism job, locally;
     - the Part 2 reduction tests over D01–D14;
     - the mutation floor check, or confirm that the last recorded run meets the floors.
  3. If any Part 2 gate fails or its evidence is missing: **stop**. Write the failure into `docs/HANDOFF_PART_03.md` under "Part 2 gate failure" and do nothing else.
  4. Write the rebase table in `docs/PART3_REBASE.md`: each Part 1 name used in this document mapped to its current name (e.g. `reduce_with_limits`, `Checker::check_selected`, shapes, bundle version, last ADR number).
  5. Verify these versions per R1 and record them in `docs/STACK.md`: rustc stable (is 1.100.0 out? It was scheduled for 2026-11-12), cargo-fuzz, the nightly date, Kani, cargo-mutants, cargo-deny, gungraun, hyperfine, samply, serde_json, rayon, the hasher candidates, mimalloc, tikv-jemallocator, sonic-rs, simd-json, arbitrary.
  6. Record in `docs/PART3_CURRENT_BEHAVIOUR.md` what each pathological input does today, including the current overflow and limit status mapping. This is the R9 baseline.
- **Crates:** none (docs only).
- **Verify:**
  - `cargo test --workspace --locked`;
  - `scripts/check_ratchets.sh` (use the repo's actual name);
  - `git status` shows nothing but docs changes.
- **Pass:** every Part 2 gate re-run green; the rebase table complete; a verified version in STACK.md for every tool used later; the current-behaviour doc written.
- **Commit:** `CP3.0: verify Part 2 gates, rebase table, stack inventory`.
- **Stop if:** any Part 2 gate fails, or the test count is below the ratchet.

### CP3.1 — Measured "before" baseline
- **Objective:** commit raw baselines of the current code before any Part 3 change.
- **Tasks:**
  1. Add `scripts/bench_record.sh`, which captures the section 13 metadata.
  2. Run the gungraun benches (Linux), hyperfine on the existing generator's 1M output, `/usr/bin/time` RSS at 100k and 1M, and the existing criterion benches.
  3. Commit everything under `evidence/part3/baseline-before/`.
  4. Run the same subset on the Mac, or mark it "pending Zakir".
- **Crates:** `qlrs-bench`, scripts.
- **Verify:** the evidence files exist and parse (`jq . file.json`).
- **Pass:** raw outputs with metadata committed for Linux; macOS status recorded.
- **Commit:** `CP3.1: baseline-before measurements`.
- **Stop if:** gungraun or Valgrind still cannot run on ubuntu-24.04 after two attempts.

### CP3.2 — Streaming generator and presets
- **Objective:** SCALE-003.
- **Tasks:** implement section 11: the RNG with test vectors, the streaming writer, digests, presets, the file-size check script and `.gitignore`.
- **Crates:** `qlrs-testkit` (and `xtask` if used).
- **Verify:**
  - `cargo test -p qlrs-testkit`;
  - running `qlrs-gen --preset s1m --seed 1` twice gives identical digests, on Linux and in macOS CI;
  - `/usr/bin/time` RSS for `s10m` ≤ 64 MiB (proposed); record the file size.
- **Pass:** digests match across runs and OSes, the RSS bound holds, and no generated file is tracked.
- **Commit:** `CP3.2: streaming generator, presets s1m/s10m/s100m, digests`.
- **Stop if:** existing generator outputs (goldens) would change. Existing generators must keep byte-identical output; the streaming presets are new entry points.

### CP3.3 — Resource limits, typed errors, bundle path safety
- **Objective:** HARD-002, HARD-003, HARD-005, HARD-006.
- **Tasks:**
  1. Implement the section 7 `Limits`, CLI flags and error IDs, plus `docs/ERRORS.md`.
  2. Add tests at limit−1, at the limit and at limit+1 for every limit.
  3. Add the pathological-input tests:
     - duplicate keys at every object position;
     - deep nesting (10,000 `[`);
     - a 2 GiB single line, produced by a streamed generator (no committed file);
     - a numeric string of 10^6 digits;
     - interner growth (a unique id per line);
     - u32 boundaries, using synthetic counters rather than huge files.
  4. Implement bundle path safety and atomic writes.
- **Crates:** `qlrs-io`, `qlrs-schema`, `qlrs-bundle`, `qlrs-cli`.
- **Verify:** `cargo test --workspace`, and `scripts/check_exit_codes.sh` shows CLI exit codes matching the section 7 table.
- **Pass:** every limit row has a test and gives its specified status, and no existing golden changes (otherwise R9 applies).
- **Commit:** `CP3.3: resource limits, typed errors, bundle path safety, atomic writes`.
- **Stop if:** a mapping would change an existing verdict or exit code (R9).

### CP3.4 — Fuzz infrastructure plus parser and num targets
- **Objective:** F01, F02, F05 and F06 running, with corpus replay on stable. F03 needs the CP3.6 chunked parser, so add it there.
- **Tasks:**
  1. The `fuzz/` workspace with its pinned nightly, plus `qlrs-testkit::oracles`.
  2. Seed corpora and the dictionary generator.
  3. The corpus-replay tests.
  4. `fuzz-smoke.yml`.
  5. ADR-E.
- **Crates:** `fuzz/`, `qlrs-testkit`, `qlrs-io`, `qlrs-num`.
- **Verify:** `cd fuzz && cargo fuzz list`; `cargo fuzz run <t> -- -max_total_time=60` for each target; `cargo test --workspace` (which runs the replay).
- **Pass:** every target builds and runs 60 s without crashes (or crashes are handled per R10), and the smoke workflow is green.
- **Commit:** `CP3.4: fuzz workspace, parser and num targets, corpus replay`.
- **Stop if:** cargo-fuzz cannot build on the pinned nightly after two attempts. Try the previous nightly once before stopping.

### CP3.5 — Checker, reducer, bundle targets and differential oracles
- **Objective:** F07–F12.
- **Tasks:** structured `arbitrary` generators built from the shapes; the section 6 oracles; seeds from D01–D14; the nightly matrix.
- **Crates:** `fuzz/`, `qlrs-testkit`.
- **Verify:** a 60 s smoke run per target, and a 20-minute nightly run per target.
- **Pass:** no unfixed crashes; every crash has a committed regression test (R10); any defect that changes a verdict is escalated (R9).
- **Commit:** `CP3.5: checker/reducer/bundle fuzz targets and differential oracles`.
- **Stop if:** an oracle disagreement implies a semantic defect in I1–I3 or the reducer. Report it with a minimal input; do not "fix" the oracle.

### CP3.6 — Streaming ingest without source retention
- **Objective:** SCALE-002, ADR-A, ADR-B.
- **Tasks:** implement section 8 (chunked parse, deterministic merge, offsets, positioned re-reads with digest check, spooling of non-seekable input); add F03; remove source retention.
- **Crates:** `qlrs-io`, `qlrs-schema`, `qlrs-reduce` (witness extraction), `qlrs-bundle`, `qlrs-cli`.
- **Verify:** all goldens and fingerprints byte-identical; F03 for 20 min; the determinism thread matrix; RSS at 1M compared with the CP3.1 baseline.
- **Pass:** outputs identical; peak RSS at 1M lower, by roughly the source size; witness lines byte-identical to the original lines.
- **Commit:** `CP3.6: streaming ingest, offsets, positioned re-reads; no source retention`.
- **Stop if:** any output byte changes.

### CP3.7 — Hot-path optimisations (10.1–10.5, 10.9–10.11)
- **Objective:** measured constant-factor improvements.
- **Tasks:** one technique per sub-commit on the checkpoint branch, squashed into one CP commit at the end. Record each delta in `docs/PERF_LOG.md`.
- **Crates:** `qlrs-check`, `qlrs-ledger`, `qlrs-reduce`, `qlrs-io`, `qlrs-schema`, `qlrs-num`.
- **Verify:** gungraun before and after each technique; goldens identical; mutation floors still met.
- **Pass:** each kept technique improves instructions or RSS by more than 1% (proposed noise floor); reverted techniques are documented.
- **Commit:** `CP3.7: hot-path optimisations with measured deltas`.
- **Stop if:** an optimisation needs a semantic change.

### CP3.8 — Parser strategy decision
- **Objective:** ADR-C and section 9.
- **Tasks:** build the safe scanner experiment; add F04; measure the SIMD crates bench-only; decide using the section 9 criteria.
- **Crates:** `qlrs-io`, `qlrs-bench`, `fuzz/`.
- **Verify:** if the scanner is adopted, F04 has run for ≥ 1 CPU-hour with logs; instruction counts per event recorded for each parser.
- **Pass:** ADR-C written with numbers. The scanner is either adopted under all criteria, or left disabled and documented.
- **Commit:** `CP3.8: parser strategy decision (ADR-C)`.
- **Stop if:** the scanner and reference diverge on an input and SPEC is ambiguous about the cause. Escalate.

### CP3.9 — Allocator evaluation
- **Objective:** ADR-F.
- **Tasks:** implement 10.6 behind a feature; measure 1M and 10M on Linux and macOS; decide.
- **Crates:** `qlrs-cli`, `qlrs-bench`.
- **Verify:** raw outputs committed.
- **Pass:** decision recorded. If an allocator is adopted, cargo-deny and the supply-chain docs are updated and outputs are byte-identical.
- **Commit:** `CP3.9: allocator evaluation (ADR-F)`.
- **Stop if:** any allocator changes any output.

### CP3.10 — Scale runs and determinism at scale
- **Objective:** SCALE-001, SCALE-004, DET-001.
- **Tasks:**
  1. `s1m` and `s10m`: generate, verify, and reduce the injected-defect variants (early, middle, late).
  2. Record time and peak RSS on ubuntu-24.04, ubuntu-24.04-arm and macos-15 (or Zakir's Mac).
  3. Run the thread-count matrix and the cross-OS digest comparison.
- **Crates:** none new (scripts, CI).
- **Verify:** `scripts/scale_run.sh s10m`, with outputs in `evidence/part3/scale/`.
- **Pass:** every run completes; every witness replays to VIOLATION with the same fingerprint; digests are identical across threads and OSes; RSS is within the proposed gates, or the miss is documented with numbers.
- **Commit:** `CP3.10: 1M/10M scale and determinism evidence`.
- **Stop if:** any output differs between OSes.

### CP3.11 — Kani and Miri
- **Objective:** FV-001.
- **Tasks:** harnesses K01–K07; the Miri job; solver times and bounds recorded.
- **Crates:** `qlrs-num`, `qlrs-schema`, the test-allocator crate.
- **Verify:** `cargo kani -p qlrs-num` (and `-p qlrs-schema`); `cargo +nightly miri test -p qlrs-num`.
- **Pass:** every harness verifies at its documented bound, or a reduced bound is justified in the ADR; Miri is clean.
- **Commit:** `CP3.11: Kani harnesses and Miri`.
- **Stop if:** Kani finds a counterexample. Commit it as a regression test (R10), and escalate if it is semantic.

### CP3.12 — Mutation ratchet
- **Objective:** section 15.
- **Tasks:** nextest integration, the diff job, the sharded nightly run, floors for each new area, the allowlist.
- **Verify:** a summary of a full sharded run's `outcomes.json` is committed.
- **Pass:** floors are recorded and are at least the previous floors; the PR diff job is green.
- **Commit:** `CP3.12: mutation ratchet with shards and in-diff`.
- **Stop if:** any existing floor would drop.

### CP3.13 — Supply chain and security review
- **Objective:** SUP-001 and `docs/SECURITY_REVIEW_PART3.md`.
- **Tasks:** section 17 items 1–4, 6 and 7 (item 5 only if ADR-J has happened); the security review document.
- **Verify:** `cargo deny check`; `cargo auditable build --release` produces a binary with embedded dependency data (check with `cargo audit bin` if cargo-audit is installed).
- **Pass:** cargo-deny green in CI; review document complete; actions pinned.
- **Commit:** `CP3.13: supply-chain policy and security review`.

### CP3.14 — CI hardening and perf gate
- **Objective:** section 16 complete, with PERF-001 and PERF-002 active.
- **Tasks:** the workflows; the ARM fallback rule; artifact uploads; the committed after-baseline (`evidence/part3/baseline-after/`).
- **Verify:** one green run of each workflow (nightly and weekly via manual dispatch).
- **Pass:** every workflow is green, and the perf gate demonstrably fails on a deliberate +5% regression in a throwaway branch (logged, not merged).
- **Commit:** `CP3.14: CI workflows, perf and RSS gates, after-baseline`.

### CP3.15 — Long fuzz campaigns, Gate S evidence, handoff
- **Objective:** Gate S.
- **Tasks:**
  1. Fuzz each target for ≥ 1 CPU-hour (`fuzz-long.yml` or locally).
  2. Commit trimmed logs, ≤ 1 MiB each: command, header, stats every 5 minutes, final stats.
  3. Minimise and commit the corpora.
  4. Fill in `docs/HANDOFF_PART_03.md`.
  5. Run the Gate S checklist.
- **Verify:** `scripts/gate_s.sh` prints PASS/FAIL per criterion, using committed evidence only.
- **Pass:** every Gate S criterion is PASS.
- **Commit:** `CP3.15: Gate S evidence and handoff`. Tag `part3-gate-s` only if every criterion passes.
- **Stop if:** any criterion fails. Do not tag; report.

Optional CP3.B: only after CP3.1, and only if Rust 1.100.0 has actually been released.
1. Bump the toolchain per ADR-J.
2. Re-run everything and re-baseline under `evidence/part3/baseline-1.100/`.
3. Enable the cooldown (section 17.5).

## 19. Gate S — scale and hardening exit criteria

Every criterion is measured from committed evidence, by `scripts/gate_s.sh`:

| # | Criterion | Evidence |
|---|---|---|
| S1 | Every section 6 target (F04 and F11 only if they exist) has ≥ 3600 CPU-seconds in one campaign, run on the Gate S commit or on a commit whose fuzzed crates are unchanged since, with zero unfixed crashes, timeouts or OOMs | `evidence/part3/fuzz/<target>/*.log` with final stats |
| S2 | Every crash found in Part 3 has a passing regression test | `tests/regressions/fuzz/**` plus a test list |
| S3 | `verify` on `s10m` completes on ubuntu-24.04 and on Apple Silicon (CI macos-15 or Zakir's Mac), with wall time and peak RSS recorded | `evidence/part3/scale/` |
| S4 | `reduce` completes on the `s1m` and `s10m` injected-defect traces (early, middle, late). Each witness is lifecycle-valid, replays to VIOLATION with the same fingerprint, and has lines byte-identical to the source | `evidence/part3/scale/reduce_*` |
| S5 | The section 13 peak-RSS gates are met, or each miss is documented with numbers and approved by Zakir | `/usr/bin/time` outputs |
| S6 | Output digests are identical across `RAYON_NUM_THREADS` ∈ {1,2,3,4,8} and across Linux x86_64, Linux arm64 and macOS arm64 | determinism job logs |
| S7 | The after-baseline is committed, and the CI perf gate is active and shown to trip on a deliberate regression | `evidence/part3/baseline-after/`, CI log |
| S8 | Every section 7 limit is tested, and the exit-code script is green | test list, CI |
| S9 | Kani K01–K07 pass at their documented bounds, and Miri is clean | weekly job log |
| S10 | Mutation floors are at least the previous floors, and floors for the new areas are recorded | `mutants-floors.toml`, summary |
| S11 | cargo-deny is green and the security review is complete | CI, doc |
| S12 | The test-count ratchet is raised to at least the previous count plus every new test. Goldens and fingerprints are unchanged except for approved changes | ratchet file |
| S13 | `docs/HANDOFF_PART_03.md` is complete | file |

## 20. Handoff template: `docs/HANDOFF_PART_03.md`

```
# Handoff — Part 3 (Scale and Hardening)
Commit: <sha>   Date (UTC): <date>   Toolchain: <rustc -Vv>   Fuzz nightly: <date>
## Part 2 verification (CP3.0)
- Gates re-run: <list with PASS/FAIL and evidence path>
- Rebase table: docs/PART3_REBASE.md
## Checkpoint status
| CP | Status | Commit | Evidence path | Notes |
## Gate S
| # | PASS/FAIL | Evidence | Notes |
## Fuzzing
| Target | CPU-seconds | Execs | Corpus files/bytes | Crashes found | Regression tests | Log |
## Scale results (raw paths required)
| Host | Preset | Command | Wall time (median, hyperfine) | Peak RSS | bytes/event | Output digest |
## Performance baselines
- Instruction counts: <path>   Delta vs before: <table>
## Limits and statuses
- Final defaults (ADR-D): <table>
## Decisions (ADRs)
| ADR | Title | Decision |
## Verdict-affecting changes (each with approval reference)
## Mutation floors (before → after)
## Supply chain
- cargo-deny: <status>; cargo-vet: <status>; auditable: <status>; SBOM: <status or "not produced">
## Known gaps, unmet goals, blockers (R5)
## Interfaces guaranteed to Parts 4–5 (see section 22)
```

## 21. Definition of Done

Part 3 is done when all of these hold:
1. Gate S is PASS on every criterion, with its evidence committed.
2. Every ADR is written.
3. SPEC and TRACE contain the section 4 IDs, each linked to tests.
4. STACK.md lists only verified versions.
5. The handoff is complete.
6. No ratchet was lowered.
7. There are no unapproved verdict changes.
8. No generated trace files are in the working tree.
9. The `part3-gate-s` tag is on a fully passing commit, and nowhere else.

## 22. Interfaces Part 4 and Part 5 can rely on

A real-engine adapter (NautilusTrader Mode T, in Part 5) must be able to rely on the following.

1. **One builder for every source.** `TraceStoreBuilder` (name per the rebase table):
   - accepts events in seq order;
   - assigns interner IDs deterministically, by first occurrence;
   - enforces `Limits`;
   - produces the same `TraceStore` whether the source is qlrs JSONL or an adapter.

   Adapters must not bypass `Limits` or the interner.
2. **Provenance.**
   - Each event carries a `SourceRef { file_id: u32, offset: u64, len: u32, line_or_record: u64 }`.
   - Witness extraction re-reads source records through a `SourceReader` trait: `read_record(&SourceRef) -> Result<Vec<u8>, IoError>`, using positioned reads with a digest check.
   - Part 6 can later implement `SourceReader` for columnar files.
3. **Error taxonomy.**
   - Error IDs are stable: `E-*` input errors exit 1, `U-*` unsupported exit 4, `I-*` budget exit 3.
   - An adapter adds its own `E-NT-*`/`U-NT-*` IDs and never reuses existing ones.
4. **Resource model.**
   - The `--stats` JSON reports events, bytes/event (columns), offsets bytes, interner entries and bytes, peak RSS (when measured externally) and `heap_bytes_estimate`.
   - Adapters must report the same keys.
5. **Determinism contract.** Outputs are byte-identical across thread counts and OSes. Time-limited outputs are labelled. Hash order is never exposed.
6. **Fuzz harness pattern.** Reuse `qlrs-testkit::oracles` and the `fuzz/` workspace conventions. Each new adapter parser gets:
   - its own F-targets, seeds, dictionary and corpus-replay tests;
   - a differential target against the canonical qlrs JSONL produced from the same events.
7. **Limits.**
   - Event and index ceilings are u32: at most 4,294,967,294 events per trace.
   - The line and record size limits apply, and scale stays ≤ 18.
   - Anything an engine emits beyond these maps to `U-*` UNSUPPORTED and is never truncated.

Not guaranteed: internal column layouts, scanner internals, the allocator choice and bench names.