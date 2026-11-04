# Part 2 authoritative rebase

The user's rebase overrides the uploaded PART_02_INSTRUCTIONS.md. Starting source
281fa154ac3e2cae4a3f2155d025b91c57b30e97 has 60 tests, not 42. ADRs 0008/0009
already exist; new decisions start at 0010. Stage A is reused, not repeated.
Keep checkpoint orders/trades and --time-limit-ms / --max-calls.

CP2.0: proven VIOLATION outranks missing evidence, diagnostics remain, reduce
accepts a supported target window. Updating SPEC and req_check_011 is explicitly
authorized. Completeness belongs to full-trace verify, not scoped witnesses.
I2 witnesses exclude unrelated instruments and cash claims unless required.

Gate: per-case bound is required target-window fills + two boundary claims +
order context, or twice the exhaustive minimum where computable. Add S9 with
staggered claims which need never align. Pins must be minimal by construction;
each profile has removable units and some cases test certificate candidates.
Report candidate counts; zero is vacuous. Kill evaluate_batch/remove_closure
survivors. Numeric/checker, reducer and fingerprint mutation floors remain separate.

Performance: remove dynamic dispatch in the checker loop, precompute order
quantity limits as a column, use partition_point for window setup. Commit raw
before/after measurements under bench/results/CP2.x with actual machine metadata.
Checkpoint commits use CP2.x and completed tags part2-cp2.x. Do not create fake
checkpoints for already completed Stage A work. Existing safety, dependency,
numeric, golden and determinism protections remain in force.
