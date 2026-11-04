# Claim-boundary validation

Production source: `956790b`. These logs cover the user's ten follow-up
corrections; historical Part 1 evidence remains in the parent directory.

```sh
./ci/check-all.sh
python3 ci/catalogue.py
cargo nextest run --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RAYON_NUM_THREADS=1 cargo +nightly miri test -p qlrs-testkit --test allocations --locked
python3 ci/instruction-ratchet.py bench-results/2026-11-04-claim-boundaries-initial-linux-x86_64/instructions.txt
```

The complete routine suite includes fmt, clippy, 60 native tests without skips,
doctests, dependency/license/advisory checks, money-type/unsafe guards, protected
goldens, requirement traceability, the test-count floor and thread determinism.
`catalogue.txt` tests the actual CLI against the unchanged protected catalogue.
`miri-rustc.txt` records the installed nightly compiler used by Miri.

`hosted-ci.json` records directly observed public GitHub job statuses with their
source commit and observation time. It does not substitute local results for
macOS results. Numeric Miri edge/rounding logs remain in the parent directory
because `qlrs-num` is unchanged.

`instruction-ratchet.txt` applies the original 5% floor.
`legacy-mutation-ratchet.txt` explicitly validates the archived num/check-only
campaign using `--legacy`. Current mutation validation requires reducer and
fingerprint coverage as well; its full raw record is retained in the corrections
benchmark directory. Archived legacy coverage is not accepted as a current
full-scope campaign.

The initial full mutation campaign subsequently failed the zero-timeout gate
with three timeouts. Its failure evidence is retained in the initial benchmark
directory. It is not the passing final mutation baseline.
