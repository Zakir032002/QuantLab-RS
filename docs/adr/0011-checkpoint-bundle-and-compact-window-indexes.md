# ADR-0011: checkpoint/bundle v1 and compact window indexes

Context: the rebased Part 1 already has orders, trades, independent checkpoint
cash/positions and observed opening windows. Renaming those fields would break
existing callers. Global-length reducer CSR allocations also obscure local cost.

Decision: add schema_version=1 with absent/zero meaning legacy. Reject future
versions. Preserve orders/trades and their complete lifecycle/quantity/trade
context. Bundle v1 declares per-record Anchor, Asserted, Evidence and Synthetic
roles, original sequence/byte ranges, a certificate and a compact log. V0 reading
retains its old full-trace semantics. The scoped Checker requires an observed
opening boundary when the finding names one. Header/checkpoint initial state
remains the boundary when there is no preceding observed claim.

Use partition_point to bound graph/event scans, then compact sorted original
unit IDs and local CSR rows. Total full-input validation and independent prefix
checkpoint derivation still require original-history work. Only window indexing
and edge construction are proportional to the selected window. A global mask
still costs one bit per original unit; do not claim total O(window) setup.

The checker uses monomorphized iterators and prepared quantity-limit columns.
Prepared stores must be treated as immutable; Ledger::prepare computes the
column once. No submitted quantity precision policy changes.

Consequences: stable field names and v0 decoding remain; additive v1 proof data
is explicit. No protected catalogue, golden or fingerprint encoding changes.
