# ADR-0008: claim boundaries, bounded reduction and realistic validation

## Context

The user's ten requested corrections supersede the original global I2 and
first-retained-event checkpoint behavior. Whole lifecycle retention, repeated
pattern scans and per-call removed-unit lists are ineffective on interleaved,
long-lived orders and sparse claims. A claim that carries an earlier error is
an opening boundary, not a new independent assertion at the checkpoint.

## Decision

Cut at the target invariant's preceding claim. Preserve expected cash/positions,
referenced order metadata/status/filled quantity and relevant prior trade IDs.
Carry observed opening claim values separately from independent expected state,
with independently computed movement since each opening claim. Skip auditing
those opening claims. I2, like I1, compares consecutive same-scope claims and
pins all fills in its local window. Never change an observed line.

Construct inventory dependencies only after the cut, consuming checkpoint
inventory before post-cut buys. A fill requires a later cash claim and later
same-instrument position claim. Missing tail evidence makes the overall verdict
INCONCLUSIVE (UNSUPPORTED still wins), while complete earlier findings remain
available as diagnostics. This explicit user instruction changes that part of
the original verdict precedence; regression expectations change only where this
new rule applies. Existing protected golden values and catalogue statuses stay.

Use incremental global/per-order kind counts and snapshots at claim boundaries.
Evaluate sparse retained event indices with the complete checker instead of
maintaining a second, divergent I1-only financial implementation. Log constant
size summaries, not vectors proportional to removed history.

Bound candidate evaluations by configurable time and call budgets. Preserve the
best verified witness on exhaustion; never claim closure-1-minimality after a
limit. Limits are cooperative: mandatory input validation and a started replay
finish safely. Default limits are 30 seconds and 10000 candidate calls.

Extend generators/gates with interleaved and long-lived orders, sparse claims,
multiple instruments, partial fills and carried-forward errors. Gate D01–D11
with explicit kept-event and time bounds. Extend mutation scope to reducer and
fingerprint code, preserve the legacy score floor and record new scope floors
from actual completed campaigns.

The initial expanded mutation run exposed FIFO mutants that repeatedly
reinserted a zero remainder and timed out. Do not count those timeouts as kills
or loosen the zero-timeout gate. Consume queue entries monotonically and return
a residual lot only after consumption finishes. Extend the FIFO regression to
cover a partially consumed buy lot spanning two sells, then rerun the complete
campaign on the corrected source. A third mutant negated the final no-progress
certificate condition and repeatedly waited for the time budget. Bound that
pass by the initial removable-unit count plus one: every successful pass must
remove a unit and the extra pass checks the fixed point. The certificate still
requires an actual complete no-progress pass, and limit exhaustion stays false.

Order, trade and claim strings share one arena. A new order can reuse a symbol
first seen in another role, so order symbols are not necessarily numerically
sorted. Preserve the binary-search common path with a numeric linear fallback
and test a long-lived order whose name reuses an earlier bootstrap trade ID.
This lookup correction is outside the three broadly mutated packages; retain
the frozen broad-campaign source record and validate the added fallback with
focused schema mutations, native tests and the scale gates.

## Consequences

Checkpoint and log fields extend the v0 contract; older absent optional context
fields retain their legacy meaning. Public replay/predicate interfaces remain
fallible. Budgeted results and complete certificates are distinguished in the
CLI, bundle manifest and reproduction instructions. New requirement IDs/tests
are appended under this explicit user authorization; no tolerance is relaxed.
