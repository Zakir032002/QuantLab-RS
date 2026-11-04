# ADR-0006: exact fee rounding, evidence integrity and explicit validation scope

## Context

Independent review exposed a half-even double-rounding case: a buy of 1 at
1.005 with fee 0.01 must debit 1.02 under the declared per-fill delta profile.
Rounding notional first incorrectly debits 1.01. JSONL retention also makes the
whole-process memory target different from the compact replay-store target.

## Decision

Add checked `mul_add_rescale` with signed 256-bit alignment/add/subtract so
notional plus fee is rounded once. Verify with a separate arbitrary-precision
oracle and C09, without changing any prior expected fixture or tolerance.
Keep release overflow checks. Constant-divisor numeric rescaling dispatches to
const-generic specializations. No target-cpu=native, allocator switch or PGO.

The Linux instruction experiment with bench overflow checks disabled emitted
the same Ir counts for decimal parsing, checked multiplication and replay as
the enabled build. Both raw outputs and the exact override command are retained
under bench-results. This limited experiment supplies no measured instruction
benefit for disabling checks; it is not a claim about every workload or CPU.

Keep input bytes at the safe I/O boundary for verbatim evidence. Compact spare
column capacities and avoid storing canonical numeric strings twice. Report
column bytes and whole-process high-water memory separately; do not claim the
whole-process 100-byte/event goal has been met.

Use a detached manifest checksum in addition to hashed payloads to detect
single-byte manifest corruption. This is not an authenticity signature.
Initial protected catalogue/golden files and their manifest are established
from the user's specification and independent worked example; future changes
still need an ADR and human review.

Miri runs arithmetic edge tests, the exact-fee regression and the isolated
allocation test. The two 100,000-iteration native stress loops are filtered out
under Miri after a prohibitively slow interpreter run; their full native tests
remain mandatory and unchanged. This is disclosed bounded interpreter coverage,
not a claim that the entire high-volume suite ran in Miri. Small JSON batches
use the serial production path; this also avoids an upstream Crossbeam Stacked
Borrows failure under Miri. Large native parses and thread determinism exercise
the parallel path.

## Consequences

Functional gates and native arithmetic stress checks remain strict. Sparse I1 algebraic admission is implemented with u32 CSR event indexing and
original-lifecycle validity as an explicit premise. Dense and mixed-I3 candidates
fall back to full checks. A randomized candidate differential test and final
full replay guard against disagreements. macOS and Linux CI are declared; local
Linux validation must never be described as hosted macOS success. Checkpoint
history is delivered as an integrated implementation, not fabricated historical
green tags. All deviations and unmet engineering targets appear in the handoff.

Escaped JSON strings use an owned cold path while ordinary strings remain
borrowed. Re-bundling an already-reduced trace records checkpoint ancestry
separately from the new input commitment, without modifying synthetic state.

The pinned compiler accepts `u128::carrying_mul`; an explicit compile probe is
retained. No arithmetic rewrite was justified after the independent oracles
passed. Unknown order metadata cannot supply financial deltas or instrument
scopes: report the lifecycle issue and an evidence limitation rather than
inventing cash, position or duplicate-trade residuals. Numeric parse limitations
similarly do not turn zero placeholders into model-precondition claims. New
requirement IDs are appended with matching tests; existing IDs, expected statuses
and tolerances are unchanged. This is authorized initial implementation and
hardening under the user's instruction to complete Part 1.
