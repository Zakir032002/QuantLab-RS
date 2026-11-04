# ADR-0014: incremental cache and deterministic speculation

Decision: cache buckets use deterministic 128-bit tabulation keys updated only
for toggled IDs. Every hit compares the full canonical sorted unit list. The
predicate fixes source/window/scope context; a source/window salt is computed
once. Candidate telemetry uses contextual 128-bit hexadecimal commitments,
replacing per-candidate full-mask BLAKE3. Fingerprint bytes remain fp1, unchanged.

Use fixed batches independent of Rayon worker count. AtomicBool cancellation
only skips higher-index probes after a success. Commit the lowest successful
index and discard higher outcomes, errors and cache effects regardless of which
workers actually completed. Budget and full_calls counters reserve the whole
batch, conservatively counting canceled full-check slots. They are deterministic
upper bounds on executed candidate checks; input/context/final validation remain
separate. Actual CPU work may vary, serialized results do not.

The SBR prefix order is ascending cost. The ddmin reference uses its stable
candidate order. Failed speculative candidates cannot change the committed
witness. req_reduce_019 exercises genuine multi-candidate success with 1/2/8
workers, verifies lower-index choice, discarded cache entries and stable logs.

I2 may need another instrument's fill to fund its own buy. Do not delete all
other instruments before the oracle can check that model dependency. The saved
cash-support fixture forces a rejected numeric seed and real binary fallback;
certificate removal tests prove the funding sell is necessary. Unneeded asset
fills disappear from ordinary certified I2 witnesses. On zero budgets, a valid
narrow I2 seed is still attempted as mandatory context validation.

Small-window exhaustive-quality and shared-prerequisite regressions protect the
numerical optimizations. None of these changes weaken a protected test or alter
the financial model, catalogue, golden files, or fingerprint encoding.
