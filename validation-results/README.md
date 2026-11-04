# Validation evidence

The source baseline is `47ed446a6f3a50338b30693680184c623bb916b4`.
Bench/mutation source hashes also identify the exact tested implementation.

- `cloud-setup-repeat.txt`: successful repeat of ci/setup-cloud.sh, including the
  complete routine quality suite and protected catalogue CLI comparison.
- `check-all.txt`: the same complete run retained under the routine-check name.
- `committed-source-tests.txt`: all 42 tests after committing, with real source
  provenance embedded in the bundle build.
- `release-demo.txt`: release generation, violation verification, reduction,
  bundle verification, checkpoint replay and manifest source-commit assertion.
- `clean-million.txt`: independent clean million-event CLI verdict and digest.
- `miri-*.txt`: bounded numeric edge/rounding and allocation instrumentation
  coverage; the toolchain file identifies the actual interpreter compiler.
- `python-numeric-oracle.txt`: supplementary independent integer comparisons.
- `determinism.txt`: one/eight-thread report and entire-bundle byte comparison.
- `dependency-negative-test.txt`: intentional forbidden edge rejected, then
  reverted; this is negative-test evidence, not a remaining project failure.
- `carrying-mul-probe.txt`: successful stable compiler API probe.
- `instruction-ratchet.txt` and `mutation-ratchet.txt`: measured baseline gates.
- `hosted-ci-status.json` and `hosted-ci-current.json`: directly observed green
  jobs for the source baseline and complete evidence commit, with times/links.
  These are separate from local results and do not claim fresh restoration.
- `benchmark-packaging.txt`: bounded public-record roundtrip; raw samples retain
  their full commitment and each record fits the public HTML limit.

`numeric-foundation.txt`, machine.txt and rustc.txt preserve initial bootstrap
evidence; use the final bench-results machine/compiler files for performance.
Raw Criterion estimates/samples, Valgrind output, scale measurements and complete
mutation outcomes/survivors/source snapshots live in bench-results.

The complete tested install script and startup instructions are saved in cloud
configuration draft revision 4. Persistence was independently read back and
matched to ci/setup-cloud.sh and ci/cloud-start.md. Publishing remains a user
action in environment settings; saving did not apply or publish the snapshot.

Captured stdout is retained with its terminal newlines, including blank final
lines; source whitespace checks exclude raw result directories.
