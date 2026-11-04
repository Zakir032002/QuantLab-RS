# ADR-0010: verdict precedence and scoped witnesses

Context: the Part 2 rebase explicitly authorizes correcting SPEC precedence and
req_check_011. Previously a missing tail observation hid a proven violation.
A global cash/position frontier also retained unrelated instruments in I2.

Decision: unsupported semantics still outrank violations; proven violations
outrank inconclusive evidence, with every missing-evidence diagnostic retained.
Full-trace verify keeps the completeness rule. Add scoped Checker entry points
for witnesses, retaining target-scope financial findings, lifecycle findings and
non-completeness limitations. End a witness at its target, retaining same-scope
claims. No observed source bytes are rewritten.

Bundle writers use qlrs-bundle/v1 to identify these verification semantics;
the v0 reader retains its full-trace behavior. Checkpoint names orders/trades,
CLI flags and fingerprint encoding remain unchanged. Certificate and role fields
will extend v1 in later checkpoints before release.

The existing two-call budget test could now finish in one candidate after scope
pruning. It checks exhaustion when the cap actually fires, and does not falsely
require an otherwise completed search to be uncertified. Zero/one-call and
zero-time tests continue exercising exhaustion. No golden, catalogue, numeric
policy or ratchet is changed.

Consequences: a full trace can be a violation and still have uncovered fills.
Reduction accepts it when the target's window replays with complete scoped
support. req_check_014 and req_reduce_009 cover both previously failing cases.
