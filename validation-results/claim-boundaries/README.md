# Broad campaign and finite-loop validation

This is the frozen `111b1db` record before the order-name collision correction.
Latest source validation is in `../order-lookup/`.

Production source: `111b1dbf62cd1b4f28fa10321adc1dc4bcf4f3da`.
The initial run and its three mutation timeouts are retained separately in
`claim-boundaries-initial/` and the initial benchmark directory.

```sh
RAYON_NUM_THREADS=5 ./ci/check-all.sh
python3 ci/catalogue.py
cargo nextest run --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 ci/instruction-ratchet.py bench-results/2026-11-04-claim-boundaries-linux-x86_64/instructions.txt
```

The complete routine suite includes fmt, clippy, 60 native tests without skips,
doctests, dependency/license/advisory checks, money-type/unsafe guards, protected
goldens, requirement traceability, the test-count floor and thread determinism.
The protected catalogue is tested through the actual CLI.

Local Miri allocation logs are reused from the initial source: the ledger,
numeric core and allocator instrumentation have identical source hashes.
`miri-source-reuse.txt` records that check. Fresh final-source hosted Miri runs
numeric edges, rounding and allocation instrumentation; its observed status and
the other platform jobs are recorded in `hosted-ci.json` with the source commit.

`instruction-ratchet.txt` applies the original 5% floor. Mutation results and
complete enumeration/category lists are retained in the final benchmark record.
The full mutation ratchet requires numeric/checker, reducer and fingerprint
coverage independently. Explicit archive mode accepts only historical
num/check-only results at their frozen historical floor. A negative probe
checks that those old results cannot pass as a current full-scope campaign.
A second negative probe rejects the initial expanded campaign's three timeouts;
they cannot be silently counted as kills.
