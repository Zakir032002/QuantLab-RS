Optional Linux proof: `cargo kani --manifest-path verification/prefix-sums/Cargo.toml`.
Use kani-verifier 0.68.0 (official crates.io metadata rechecked with Part 2).
The workflow dispatch input enables installation/setup on Linux.

The lemma proves four shared closure-prefix unions equal direct distinct-unit
sums for arbitrary signed i64 contributions lifted into i128. This bounded
range cannot overflow i128. Runtime checked overflow/fallback remains covered by
native tests. The lemma does not prove full-predicate monotonicity or the entire
reducer. Kani is not installed/run in the cloud session; do not report a proof
pass without its actual job output.
