The original public-window suspicion was refuted by reading
AliveMask::intersection, which clones the receiver and intersects unit words
without accepting the caller's bounds. Public Predicate::test was already safe.
The initial failed test is retained as an incorrect-hypothesis record, not a bug
reproduction. Its assertion was corrected to the documented normalization
contract. A defensive internal-batch bounds check is retained and tested through
the private helper because those callers bypass public normalization.
