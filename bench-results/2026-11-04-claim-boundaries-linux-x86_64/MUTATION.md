# Completed mutation campaign: claim-boundary corrections

Frozen Rust source: `111b1dbf62cd1b4f28fa10321adc1dc4bcf4f3da`. The complete enumeration and all 561
outcomes are committed. Zero timeouts; the unmodified baseline succeeds.

```sh
PROPTEST_RNG_SEED=42 RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1 RAYON_NUM_THREADS=1 cargo mutants -p qlrs-num -p qlrs-check -p qlrs-reduce --copy-target true --test-workspace true --jobs 4 --timeout 60 --no-shuffle --cargo-arg=--locked --output /workspace/setup/qlrs/mutation-claim-boundaries-final
```

| Scope | Generated | Caught | Missed | Unviable | Caught/viable |
|---|---:|---:|---:|---:|---:|
| qlrs-num | 222 | 196 | 11 | 15 | 196/207 |
| qlrs-check | 198 | 151 | 29 | 18 | 151/180 |
| qlrs-reduce | 141 | 78 | 39 | 24 | 78/117 |
| legacy num/check floor | 420 | 347 | 40 | 33 | 347/387 |
| fingerprints (checker subset) | 11 | 11 | 0 | 0 | 11/11 |
| all packages | 561 | 425 | 79 | 57 | 425/504 |

Fingerprints are a subset of checker mutations, not an additional denominator.
The legacy floor rises from 270/315
to 347/387; reducer mutations do not dilute it.
Reducer and fingerprint floors are independently recorded from this completed
campaign in `ci/mutants-baseline.txt`. Timeouts are never counted as kills.

Every actual mutant build/test argv contains `--workspace` and `--locked`.
The tool's initial baseline selects the three audited packages; the separate
complete native suite also passes 60 tests without skips. cargo-mutants exits 2
when survivors remain; this is a completed campaign, not a perfect score.
Missed mutations remain disclosed in `mutation-missed.txt`; no equivalence is
assumed and the ratchets do not prove the absence of financial defects.

`mutation-mutants.json` is the full enumeration, `mutation-outcomes.json`
contains every phase result and command, and the category lists/console retain
the original tool output. Exact mutated source snapshots are in
`mutation-sources/`, indexed by `mutation-sources.txt`; all production/test
commitments are in `source-sha256.txt`.

```sh
python3 ci/mutation-ratchet.py bench-results/2026-11-04-claim-boundaries-linux-x86_64/mutation-outcomes.json bench-results/2026-11-04-claim-boundaries-linux-x86_64/mutation-mutants.json
```

Default validation requires reducer and fingerprint scope coverage. Explicit
`--legacy` mode accepts only archived num/check-only campaigns and their frozen
historical floor. A negative probe confirms that the historical archive is
rejected as current full-scope evidence.

The later order-name collision correction (`e348f56`) changes qlrs-schema and
adds a testkit regression. The six broadly audited files above are unchanged.
This broad campaign was not rerun after that addition; its workspace source
remains 111b1db. Separate final-source native/platform/scale checks and four
focused schema mutations are retained in the lookup result directory.
