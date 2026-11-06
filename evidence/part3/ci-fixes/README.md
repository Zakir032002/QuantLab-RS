# CI fixes from actual failures

The old numeric ratchet retained the last numeric benchmark name while reading
new pipeline benchmarks. Consequently the generator count overwrote replay's
numeric baseline entry. `instruction-parser-before` records that failure on the
actual new output; `instruction-parser-after` and `instruction-parser-final`
record the corrected parsing on real output. Existing numeric floors and the
5% threshold are unchanged. The separate Part 3 same-runner gate uses 3%.

The full routine script then rejected clap in qlrs-testkit, which now ships the
qlrs-gen CLI. `current-quality` preserves that failure. The rule now explicitly
permits this binary's parser and checks it stays outside the library source.
`dependency-rule-after` passes. The complete routine script's current passing
output is in ../current-quality-3; 111 tests run, zero skips. Intermediate
Clippy failures are retained; the new regression uses the existing strict lints.
