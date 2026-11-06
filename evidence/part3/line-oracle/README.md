# P3-ORACLE-002: canonical expansion at the line-byte boundary

This is a fuzz-oracle defect, not a financial or parser defect. The valid input
record is exactly the oracle's 4096-byte input cap. Canonical encoding adds
optional null fields and a newline, so feeding it back with that same cap
rejects it before the equality check. The original input is accepted correctly.

`reproduce/` records the real native libFuzzer panic (77; cargo-fuzz exits 1).
`minimize/` records cargo-fuzz tmin's attempt: no smaller crashing input was
found. `regression-before/` records the explicitly run ignored regression failing
before any correction. The input and ignored regression are committed first,
as required by R10. Resource limits remain enforced on original input.
