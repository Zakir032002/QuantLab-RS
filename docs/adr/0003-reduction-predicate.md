# ADR-0003: instance-preserving lifecycle closure reduction

Status: accepted and implemented.

## Context

Shrinking must preserve financial evidence and prevent new contradictions
from replacing the target. A minimum output witness is not a minimum engine
input reproduction.

## Options

Raw-line ddmin; order-lifecycle ddmin; dependency-closure ddmin.

## Decision

Use lifecycle and claim units with CSR dependency closures, the exact target
instance key, pinned boundary claims and lifecycle units, explicit synthetic
expected-state checkpoints and an anti-slippage predicate. Keep original
sequence values and observed bytes. Finish with a full replay and a
closure-1-minimality pass; never claim global minimality. Parallel evaluation
must choose the lowest-index success deterministically.

## Consequences

Synthetic checkpoints are computed from the independent original ledger,
not observed balances. The reducer, cache, prefix index and complete sparse I1 evaluator are implemented.
Sparse I1 admission uses original lifecycle validity plus independent cash and
position prefix sums and exact original-key membership. Dense/mixed-I3 cases
fall back to full checking. Every final witness is cross-checked by full replay. Actual measurements and three million-event gate cases
are in bench-results. Public fallible replay/predicate APIs return Result to
keep internal/numeric failures distinct from false predicates.
