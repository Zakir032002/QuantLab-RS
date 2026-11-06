# ADR-0023: fill storage, deterministic lookup and explicit time budgets

Status: implemented; local native and routine checks pass. Gate S is incomplete.

Cash deltas belong to fills. Storing a 16-byte delta for every non-fill event
wasted memory, so `TraceStore.delta_cash` now uses the fill payload index, like
`fill_qty`. Preparation, replay, checker, checkpoints and reducer contributions
share that index. Arithmetic and evidence ordering are unchanged.

The builder maps dense interned order symbols through a sentinel-indexed vector
instead of a second hash table. The interner uses foldhash FixedState and a
String arena; validated UTF-8 is not validated again on every lookup. Neither
output nor IDs depend on hashing. Header validation belongs in the shared
builder. Source scanning checks spans at newline boundaries and block ends;
strict memory budgets are checked before growing the offset index. The legacy
uncapped read retains its memory-parse fallback for a line exceeding a u32 span.

The `--no-time-limit` flag supplies Duration::MAX while keeping the existing
`--time-limit-ms` and `--max-calls` interfaces. A configured finite time limit
adds `determinism: "time-limited"` to new manifests, as Part 3 section 12 requires.
Legacy manifests without this optional field remain readable. Call-only runs
omit it. This is metadata; verdicts and fingerprint bytes are unchanged.

The raw instruction and RSS records are in evidence/part3/hot-path, with exact
source hashes. The new generator is a CLI shipped by testkit: the dependency
rule permits clap there and separately rejects clap in its library source.
The core-crate dependency rules remain unchanged. No unsafe code or dependency
version is added by these changes.
