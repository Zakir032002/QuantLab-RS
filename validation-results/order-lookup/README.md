# Final order lookup correction

Source: `e348f562128c59c337590869544afd34aa91bfdf`.
An order may reuse a string first interned as a trade/claim ID, so order symbol
IDs need not be sorted. Numeric fallback lookup preserves checkpoint state in
this case. The long-lived-order regression exercises that collision through
reduction and checkpoint replay.

```sh
RAYON_NUM_THREADS=5 ./ci/check-all.sh
python3 ci/catalogue.py
cargo nextest run --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The complete routine suite passes 60 native tests without skips, all dependency,
license/advisory, protected golden, traceability and count guards, plus full
one/eight-thread report/bundle comparisons. The catalogue is tested through the
actual CLI. Instruction counts pass the original 5% floor.

Hosted statuses in `hosted-ci.json` identify this exact source. The fresh Miri
job checks numeric edges, rounding and allocator instrumentation.
The final Linux/Mac performance records use this source and retain all samples.

The initial ARM jobs were cancelled for runner capacity. The standard Intel
retry at 5e1faad passed all five jobs, including both platform quality jobs;
its Rust/manifest hashes match e348f56. Provider diagnostics and cancelled
statuses remain in separate files. Current Mac benchmarks identify Intel
hardware; earlier M1 measurements retain their original source provenance.

The broad mutation record in the claim-boundaries directory belongs to frozen
workspace source 111b1db. All six audited source files remain byte-identical;
`broad-mutation-source-match.txt` in the latest Linux record verifies that fact.
The later schema helper is covered by four focused mutations, all caught with
zero timeouts. The broad campaign was not rerun after this schema-only addition;
its source provenance and measured floors remain explicit.
