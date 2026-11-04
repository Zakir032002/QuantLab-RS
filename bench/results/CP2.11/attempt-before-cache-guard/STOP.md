This attempt was interrupted while investigating a suspected public-predicate
cache-window bug. That public diagnosis was refuted: AliveMask::intersection
already preserves the receiver's fixed bounds. The initial test expected an error
for a public mask that is intentionally normalized, so its assertion was wrong.
Do not describe that failed probe as evidence of a public correctness bug.

The final regression verifies public normalization and independently checks a
new defensive precondition on private batches, which bypass normalization.
No partial campaign score is reported as a completed result. The complete
campaign uses the final test and internal guard.
