# Part 1 domain and implementation contract

The full supplied specification is preserved in
[PART_01_INSTRUCTIONS.md](PART_01_INSTRUCTIONS.md). This file records the numeric,
trace, ledger, checker, reduction and bundle contracts implemented in Part 1.
Validation evidence and remaining platform limits are in HANDOFF_PART_01.md.

- REQ-NUM-001: decimal strings only; explicit per-value scale 0..18; reject excess scale.
- REQ-NUM-002: signed raw integer limits and checked arithmetic; no overflow panics.
- REQ-NUM-003: explicit Exact, HalfEven, HalfUp and toward-zero Down policies.
- REQ-NUM-004: cold 256-bit multiplication/rescaling when an intermediate is wide.
- REQ-NUM-005: numeric comparison by exact upscaling, including large values.
- REQ-NUM-006: decimal formatting/parsing preserves raw value and declared scale.

## End-to-end requirements added for the implemented thin slice

- REQ-BUNDLE-001: deterministic roundtrip and every file tamper.
- REQ-CHECK-001: catalogue statuses and invariants.
- REQ-CHECK-002: clean generated consistent.
- REQ-CHECK-003: all defects 1000 injection points.
- REQ-CHECK-004: residual relations are hints.
- REQ-CHECK-005: clean corpus and flat position.
- REQ-CHECK-006: unknown event is unsupported not input error.
- REQ-CHECK-007: partial and status precedence.
- REQ-CHECK-008: lifecycle subcodes and supported terminal states.
- REQ-CLI-001: exit codes and bundle commands.
- REQ-FP-001: canonical vector and order independence.
- REQ-FP-002: at seq filters status and anchor.
- REQ-IO-001: strict unknown fields decimal types and sequences.
- REQ-IO-002: roundtrip verbatim.
- REQ-IO-003: verbatim noncanonical decimal evidence.
- REQ-IO-004: malformed overflow still input error.
- REQ-LEDGER-001: zero allocations in replay loop.
- REQ-NUM-007: mul rescale matches bigint.
- REQ-NUM-008: parse format roundtrip.
- REQ-NUM-009: parser no panics.
- REQ-NUM-010: checked mul exact product scale.
- REQ-REDUCE-001: three defects exact replay and idempotence.
- REQ-REDUCE-002: actual closure minimality certificate.
- REQ-REDUCE-003: property preserves instance and source.

## Trace contract: qlrs-trace/v0

UTF-8 without BOM; each record including the last ends in LF. A strict header
comes first. Subsequent records have strictly increasing u64 `seq` and
non-decreasing u64 Unix-nanosecond `ts_event`, `account_id`, and
`provenance:"observed"`. Sequence values, IDs, whitespace and observed decimal
strings survive witness writing. Checkpoints are separate, labelled files;
synthetic records masquerading as observed input are rejected.

Header fields and event fields are exactly those in section 8.2 of the supplied
specification. The rounding field is a string: `Exact`, `HalfEven`, `HalfUp` or
`Down`, with target precision taken from the declared currency. Missing
`commission_max_scale` defaults to currency precision. Unknown fields on known
records are input errors. An unknown event kind with valid common fields is
UNSUPPORTED, so corporate actions cannot be misreported as accounting bugs.
Multiple accounts/currencies, non-CASH models, non-MARKET orders, out-of-model
scales, foreign commissions, negative fees, expected negative cash or expected
negative positions are UNSUPPORTED. Malformed JSON and decimal grammar always
win over model verdicts, including overflowing strings with invalid suffixes.

The in-memory spine preserves original sequence, timestamp, kind, unit and
payload index columns. Per-kind columns and independent deltas live in schema;
the ledger and checker do not depend on I/O. Interned strings use one byte arena
with collision-safe chains. Canonical decimal claim strings are reconstructed
losslessly from raw/scale; noncanonical strings (leading zeros or negative zero)
are retained verbatim. Original source lines are always copied unchanged.

## Exact arithmetic and rounding boundary

`Fixed` stores private i128 raw and u8 scale, with scale <= 18. Structural equality
includes scale. `numeric_cmp` uses exact upscaling, with wide fallback. Parsing
never accepts JSON numbers, exponent notation, leading plus, NaN or infinity.
Intermediate multiplication scales can reach 36; the result is returned only
at a supported declared scale. All add/subtract/multiply operations are checked.

The synthetic producer's non-Exact profile rounds **the combined signed
notional minus commission once per fill**, to currency precision. It does not
round notional separately from commission. `mul_add_rescale` performs the exact
wide sum before final rounding. The ledger precomputes that delta, and local
claim comparisons follow the same profile. This is an explicit synthetic
contract, not a claim about an external engine's rounding boundary.

Exact profiles retain commissions at the larger of currency precision and
commission_max_scale; values are compared exactly, and inexact reductions of
cash precision are INCONCLUSIVE (`rounding-policy-required`). Fractional fees
may accumulate to an exactly representable claim. HalfEven ties go to an even
last digit, HalfUp ties go away from zero, Down goes toward zero. No epsilon or
automatic tolerance exists. Final overflow is INCONCLUSIVE (`numeric-overflow`).

For future import, NautilusTrader's documented high-precision Price/Money range
at 16 decimal places is approximately -17,014,118,346,046 through
17,014,118,346,046; unsigned Quantity reaches approximately 34,028,236,692,093.
Formal import range/semantics validation remains Part 4, not an adapter claim.

## Invariants and overall precedence

I1 compares consecutive observed cash claims. The predecessor of the first
claim is starting cash or synthetic expected state. The local residual is
`(observed_now - observed_previous) - sum(delta_cash_in_window)`. Observed claims
never become independent global ledger state; they are used only as the two
observations in this local equation. Final independent cash is a derived summary.

I2 compares each instrument's claim to initial/checkpoint position plus signed
fills. A flat position after closing is valid. I3 uses the public transition
table for missing/submitted/accepted/partial/filled/canceled/rejected states,
checks total quantity and enforces trade-ID uniqueness per instrument. Stable
subcodes include unknown-order, fill-before-accept, overfill,
event-after-terminal, duplicate-trade-id and illegal-transition.

Precedence is input error, UNSUPPORTED, VIOLATION, INCONCLUSIVE, CONSISTENT.
Unsupported traces suppress all financial accusations by default. `--partial`
permits supported-prefix violations while keeping overall UNSUPPORTED. Complete
earlier contradictions can coexist with a later INCONCLUSIVE limitation.
Missing cash/position observations for retained fills are INCONCLUSIVE.

## Fingerprint encoding: qlrs-fp1

The exact instance key is invariant, subcode, account, currency or instrument,
original anchor sequence, residual raw decimal integer string and residual
scale. The class fingerprint is `qlrs-fp1:` followed by lower-case BLAKE3 hex over:

1. invariant string; subcode string;
2. u32 number of distinct event-kind entries, then sorted entries consisting of
   a u8 Kind discriminant and u64 occurrence count;
3. signed residual sign as one i8 two's-complement byte;
4. residual relation string.

Strings are UTF-8, prefixed by byte length as u32 little-endian. All integer
fields are little-endian; entries with zero count are absent. Kind tags 0..7
are Submitted, Accepted, Rejected, Filled, Canceled, CashClaim, PositionClaim,
Unsupported. No host-width integer or JSON encoding participates in this hash.

Test vector: I1 / `cash-conservation` / {(Filled,1),(CashClaim,1)} / -1 / `-fee`.
Canonical hex is committed in `fixtures/golden/fingerprint-vector.hex`; digest:
`d36cbaabf8cae916908640e54736cc3856466a66ffc4e3d37f08f9f55d373a72`.

Relations include signed fee, signed small multiples of last price, signed
notional and scale-error hints. For D10, observed is 10^m times expected and
residual is therefore (10^m - 1) times expected; the tag explicitly says
`scale-error:observed=+10^m*expected`. Relations are triage hints, never diagnoses.

## Reduction and bundle verification

Lifecycle units contain all records for one client_order_id; each claim is its
own unit. Header and instruments are pinned. I1 pins window boundaries and
lifecycle units inside its window. I3 pins the offending order; I2 pins its
claim. Additional post-fill cash/position claims are pinned when needed to keep
evidence complete rather than INCONCLUSIVE. CSR edges connect conservative buy
prerequisites to sells. A removal closes transitively over dependents.

Stage A slices to pins and prerequisites; Stage B tries deterministic ddmin
subsets/complements in parallel; Stage C checks every remaining removable
single closure until none succeeds. Lowest-index success wins. Candidate masks
are hashed as u64 unit count followed by little-endian u64 words. A foldhash map
caches outcomes. Every call, including hits and the final full replay, is logged.

Sparse I1 candidates from originally lifecycle-valid traces use a complete
precomputed evaluator: selected events come from a u32 CSR unit index, cash
and position prefix sums use checked deltas, and every retained claim residual
must match an original instance. Whole-unit deletion cannot create a lifecycle
breach in an originally legal source. Dense candidates and sources with I3
findings use full checking. Final fast and full admission must agree, and a
full witness replay always runs.
Prefix snapshots occur every 4096 original events. A checkpoint replaces only
a removed prefix and is computed from original expected state, not claims.
Predicate errors remain fallible (`Result<Outcome, String>`); internal failures
cannot silently become cache misses. Part 2 must preserve this error distinction.

Bundles use `qlrs-bundle/v0` with the files in the supplied contract and an
additional `manifest.blake3` detached commitment. Paths are allowlisted, unique,
sorted regular files; traversal, symlinks and unexpected files are rejected.
Verification rehashes every byte, checks canonical manifest encoding, replays
with the labelled checkpoint, and requires the exact finding and instance key.
This is integrity checking, not author authentication or proof of removed history
without access to the original trace. The original digest is a source commitment.

- REQ-IO-005: cash only header needs no instruments.

- REQ-NUM-011: notional fee round once.

- REQ-NUM-012: product plus fee bigint oracle.

- REQ-NUM-013: error reason codes.

- REQ-REDUCE-004: sparse fast predicate matches full replay.

Already-reduced inputs retain their checkpoint provenance digest separately
from the current observed input's original_trace digest/length in a new bundle.
Re-reducing with --checkpoint preserves the observed witness and synthetic
state byte-for-byte while truthfully recording the new input commitment.
JSON strings borrow input storage in the normal path; escaped strings use a
Cow owned cold path. Source escape spellings still survive verbatim writing.

- REQ-IO-006: escaped strings use owned cold path and roundtrip.

- REQ-CHECK-009: unknown fill never invents a cash delta.

- REQ-GRAPH-001: sell closures consume earlier buy inventory.

Sell dependencies consume earlier buy inventory in FIFO order, so later sells
do not reuse quantities already consumed by earlier sells. Synthetic opening
positions need no invented buy unit. Full financial replay remains authoritative.

- REQ-CHECK-010: Trade uniqueness requires a known instrument scope; unknown orders do not invent duplicate-trade violations.

- REQ-IO-007: Numeric limitations do not fabricate domain failures from placeholder values.

- REQ-LEDGER-002: Independent replay ignores observed claims and rejects unprepared or unresolved fill metadata.
