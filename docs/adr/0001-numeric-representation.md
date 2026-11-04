# ADR-0001: exact per-value fixed-point numbers

Status: accepted and implemented.

## Context

Prices, quantities and commissions have different declared scales. A fixed
currency-wide scale can lose fractional commissions. Intermediate products
can exceed a signed 128-bit integer even when the final rounded value fits.

## Options

Floating point; a runtime decimal dependency; arbitrary-precision runtime
integers; or checked i128 values with a minimal cold wide product.

## Decision

Use private `Fixed { raw: i128, scale: u8 }` fields with checked construction
and scales 0 through 18. Parsing allocates nothing and rejects exponent,
plus-sign and non-decimal syntax. Keep declared trailing digits. Structural
equality includes scale; `numeric_cmp` compares values using upscaling.

Multiplication first tries checked i128. On overflow, multiply unsigned
magnitudes into a (high, low) u128 pair using bounded 64-bit partial products.
Divide that 256-bit value by the required power of ten using a cold binary
long-division routine. Round only under Exact, HalfEven, HalfUp or Down.
Down is toward zero; HalfUp ties are away from zero. Every final value is
range-checked, including i128::MIN. Arithmetic errors are returned, never
converted into financial violations here.

## Consequences

No runtime dependencies, unsafe code or floating-point money operations.
Most products stay in checked i128. Rare wide division intentionally trades
latency for simple auditable logic. Constant-divisor dispatch is implemented. Linux timings are recorded in bench-results;
there is no unconditional or cross-machine speed claim.

Python arbitrary-precision integer differential checks supplement unit tests;
the specified Rust num-bigint oracle and proptest checks are also implemented.
Miri coverage is bounded explicitly in ADR-0006.
Overflow checks stay enabled; PGO is deferred to Part 17. The full ledger
must separately define rounding boundaries and intermediate common scales.
