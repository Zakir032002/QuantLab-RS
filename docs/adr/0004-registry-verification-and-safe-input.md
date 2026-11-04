# ADR-0004: continue with authoritative registry evidence and safe buffered input

## Context

The user explicitly authorized full access and instructed completion after the
previous stop. The actual proxy still denies docs.rs. Permission and runtime
network enforcement are separate. The official crates.io API, sparse registry,
source archives and upstream GitHub sources are available.

## Options

Stop again; bypass network enforcement; or verify releases against accessible
authoritative sources and disclose the unavailable second documentation check.
Unsafe mmap also conflicts with the mandatory no-unsafe rule.

## Decision

Continue using official registry release dates, checksums and source archives,
with Cargo.lock and package verification intact. Record docs.rs checks as
blocked, not verified. Do not bypass the proxy. Use safe buffered input and
borrowed record deserialization instead of unsafe mmap. foldhash 0.2.0 is the
latest non-yanked stable registry release; retain this mature release despite
its age, subject to advisory checks. Omit unused optional dependencies.

Use Python and num-bigint only as development oracles. Cargo ignores explicit
bench-profile panic settings; omit the ignored setting (benches unwind by
default). Keep release overflow checks enabled. No PGO in Part 1.

## Consequences

Dependency provenance is verifiable, but docs.rs and hosted CI remain distinct
unverified capabilities until available. Retained source bytes are accounted
separately from the compact replay store. No claimed mmap speedup.
