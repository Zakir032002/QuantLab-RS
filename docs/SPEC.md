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

I2 is local to consecutive observed position claims for the same instrument.
Its residual is `(observed_now - observed_previous) - sum(signed fills in that
window)`. Starting/checkpoint position supplies the predecessor before any
observed claim. Each observed claim becomes only the next local boundary;
independent global positions remain separate. A carried-forward offset therefore
produces one local contradiction, not repeated false accusations at later claims.
A flat position after closing is valid. I3 uses the transition table and checks
filled quantity plus per-instrument trade uniqueness, seeded by checkpoint order
status, filled quantity and relevant prior trade IDs.

Full-trace verification requires every fill to have a strictly later account
claim AND a strictly later same-instrument position claim. Earlier observations
do not satisfy this rule. Overall precedence is input error, UNSUPPORTED,
VIOLATION, INCONCLUSIVE, CONSISTENT. Missing later evidence remains in findings
even when a proven violation determines the verdict.

Witness verification checks only the target financial scope, plus lifecycle and
ledger validity. Full-trace evidence completeness does not apply inside a witness.
An I2 witness needs neither other instruments' fills nor a cash claim.
`--partial` permits supported-prefix findings while keeping UNSUPPORTED.
The Part 2 rebase explicitly authorizes this correction (ADR-0010), superseding
the earlier precedence decision in ADR-0008.

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

Lifecycle units group events for one client_order_id within the observation
window; each claim is a separate unit. The observed prefix/suffix can be clipped
without fabricating order events. A checkpoint cuts at the target I1/I2 window's
preceding same-scope claim, or the latest preceding claim for I3. Its expected
cash/positions come only from the independent ledger. Its referenced order
metadata, status and filled quantities permit continuing long-lived orders.
Relevant prior trade IDs preserve cross-boundary uniqueness. Header/instruments
and the two same-scope boundary units remain pinned. Contributing fills are
evaluated by exact residual equality; they are not hard-pinned.

Observed opening cash/position values are separate `cash_window` and
`position_windows` context, each with its original sequence and independently
computed pending movement since that observation. They do not replace independent
state. The opening claim at the cut is not audited again. This context preserves
carried errors without inventing new violations. Context fields absent from old
checkpoints retain their legacy initial-state meaning.

The observation ends at the target event. Only same-scope boundary claims are
retained; unrelated evidence schedules do not extend the window. CSR
inventory dependencies consume checkpoint positions first and only reference
post-cut buys for uncovered sells. Removing units closes over dependents.

SBR seeds nonzero algebraic contributions and their prerequisites. A rejected
seed uses cost-sorted binary prefix unions; ddmin-closures is the explicit
reference algorithm. Small exact financial windows also search cheaper numerical
subsets, preserving checked residuals before full-oracle admission. A repeated
single-closure pass certifies the final kept set relative to boundary pins.
Do not assume global monotonicity or claim global minimality on large windows.

Numerical columns and prefix unions use checked i128 arithmetic; nonlinear
rounding, I3 and overflow fall back to full checking. Cache hits compare canonical
kept-unit lists in addition to a deterministic incremental 128-bit tabulation key.
Scope/window identity is fixed per predicate. The full checker verifies every
accepted result and the final witness. Dense/sparse and independent numerical
properties protect both paths.

Checkpoint v1 adds schema_version and preserves orders/trades. Bundle v1 records
Anchor/Asserted/Evidence/Synthetic roles, original byte ranges, compact logs and a
closure certificate. Legacy v0 reading retains full-trace verification. Optional
certificate rechecking rebuilds dependencies and reproduces every rejection.
Zero tested removable candidates is explicitly vacuous. Exhausted budgets return
partial certificates and closure_1_minimal=false.

Default limits remain 30 seconds and 10000 candidate calls including cache hits.
Keep --time-limit-ms and --max-calls. Mandatory input, context and final validation
are outside search caps; a zero budget returns the verified scoped window.
Only wall-clock exhaustion is permitted to produce different cutoffs across runs.

The rebased gate covers D01-D11 on S1-S9 with actual event counts recorded for
requested 1k/10k/100k budgets and seeds 1/2/3. Per-case bounds are target-window
fills plus two boundary claims plus order context, or twice the exhaustive
minimum where computable. Small windows use the stricter exhaustive comparison.
No universal 32-event cap is imposed on sparse windows. Each generated profile
has removable context; some cases must test real certificate candidates.
The historical Part 1 rich/six-event gates remain active separately.

Bundles retain detached manifest.blake3 commitments, strict canonical manifests,
allowlisted sorted regular files and exact full-oracle finding verification.
Integrity detects corruption; it does not authenticate an author or prove removed
history without the original trace. The original digest remains a commitment.

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

- REQ-CHECK-011: every fill needs later cash and same asset position.

- REQ-CHECK-012: i2 uses local claim windows for carried errors.

- REQ-CHECK-013: sparse indices must be unique and in sequence order.

- REQ-CHECKPOINT-001: earlier bad claim is an opening boundary.

- REQ-CHECKPOINT-002: open and terminal orders and prior trades survive cut.

- REQ-CHECKPOINT-003: invalid context never becomes a financial accusation.

- REQ-CLI-002: limits preserve verified output without minimality claim.

- REQ-FP-003: all fields have distinct canonical encodings.

- REQ-FP-004: utf8 lengths sign and zero multiset entries.

- REQ-GATE-001: all eleven defects on rich profiles.

- REQ-GENERATOR-001: interleaving sparse claims long lived and carried offsets.

- REQ-GENERATOR-002: rich statuses across seeds.

- REQ-GENERATOR-003: each instrument has independent buy sell cycles.

- REQ-GRAPH-002: checkpoint inventory replaces historical buy dependencies.

- REQ-REDUCE-005: limits return best verified uncertified bundles.

- REQ-REDUCE-006: sparse dense and incremental patterns match on all defects.

- REQ-REDUCE-007: other lifecycle breaches are not admitted or returned on limits.

- REQ-REDUCE-008: short logs read legacy removed unit lists.

- REQ-CHECK-014: a proven violation outranks uncovered tails and remains reducible.
- REQ-REDUCE-009: I2 witnesses retain only target-scope position evidence.
- REQ-REDUCE-010: R1 D04 sizes finish within the existing bounded search.

- REQ-REDUCE-011: cache forced collision and incremental toggles.

- REQ-REDUCE-012: long-lived and second carried-error regressions.

- REQ-REDUCE-013: algebra agrees with the independent full checker per shape.

- REQ-REDUCE-014: evaluate_batch selects valid results and enforces caps.

- REQ-REDUCE-015: remove_closure preserves dependencies, pins and no-op behavior.

- REQ-GATE-002: nine profiles exercise removals and offline closure proofs.

- REQ-BUNDLE-002: v1 roles and rejection certificates reject false proofs.

- REQ-BUNDLE-003: original v0 writer bundles remain verifiable.

- REQ-REDUCE-016: renaming units preserves observed witnesses.

- REQ-REDUCE-017: saved nonmonotone supersets do not invalidate validated seeds.

- REQ-REDUCE-018: graph commitments and RLE have canonical encodings.
