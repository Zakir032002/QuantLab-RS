# ADR-0002: per-kind columnar trace layout

Status: accepted and implemented with safe buffered input (ADR-0004).

## Context

JSONL is an interchange format, not an efficient representation for repeated
ledger replay or reduction. Observed lines must survive reduction verbatim.

## Options

An owned event enum, a JSON DOM, or per-kind structure-of-arrays tables.

## Decision

Follow Part 1: intern IDs to dense u32 values, retain an ordered event spine
and per-kind payload columns, precompute cash deltas, and preserve source
line ranges at the I/O boundary. Keep TraceStore in schema so the ledger and
checker cannot depend on the JSON reader. Avoid unsafe mmap unless an ADR
explicitly resolves the no-unsafe requirement; buffered input remains an option.

## Consequences

The complete memory footprint, including retained source bytes, is reported
separately from compact columns in bench-results. The whole-process memory
engineering target remains unmet; column allocation and parsing were measured.
