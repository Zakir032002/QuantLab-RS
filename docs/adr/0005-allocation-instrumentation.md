# ADR-0005: isolated counting allocator in a development test only

## Context

Part 1 requires an allocation-count test of the replay loop. Implementing Rust's
GlobalAlloc trait requires unsafe declarations even when forwarding directly to
System. Money/core crates must retain forbid(unsafe_code). An evaluated safe
wrapper dependency (allocation-counter 0.8.1, published 2023-09-21) lacked the
required recent-maintenance evidence, so it was not added.

## Options

A new unverified dependency; inferred allocation freedom; or a tightly scoped
instrumentation exception covered by Miri as the specification permits.

## Decision

Allow unsafe code only in the development test binary `tests/allocations.rs`,
where a counting GlobalAlloc forwards the identical pointer/layout to System.
All implementation crates keep forbid(unsafe_code). Counters use constant
thread locals so unrelated harness-thread activity cannot affect the result.
Run this test under Miri. No runtime allocator or optimized money operation
uses unsafe code.

## Consequences

The measured region excludes initial parsing, delta preparation and state/mask
allocation. It includes repeated complete replay into existing buffers. Any
allocation/reallocation in that region fails the test. This is an explicit
instrumentation exception, not a relaxation of core safety requirements.
