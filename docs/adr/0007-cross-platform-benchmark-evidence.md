# ADR-0007: measured macOS evidence from the hosted runner

## Context

The cloud workspace runs Linux, while the specification requests Criterion
measurements on Linux and macOS. The repository has an available macOS arm64
runner. Public Actions job status and annotations can be read without account
credentials; downloading ordinary artifacts may require an authenticated session.

## Options

Leave macOS unmeasured; ask for another machine or credentials; or collect the
same seven benchmarks on the hosted runner and publish the explicit raw record.

## Decision

Run the pinned Criterion workload on macos-15 with 50 samples. Retain compiler,
CPU/OS, source commit/hashes, actual fixture digests, commands, console and every
estimate/sample/benchmark/tukey JSON file. Upload an ordinary artifact and emit
the identical compact ZIP as one public notice with its SHA-256 digest.

The packaging script accepts only those benchmark files and explicit machine
commands. It never dumps environment variables, credentials, caches, home
directories or arbitrary workspace files. Bound the notice below the platform
annotation size limit. Validate the archive commitment before retaining files.
The package smoke check round-trips all seven raw sample sets locally.

## Consequences

Both-platform measurements are reviewable without adding credentials or Rust
dependencies. Hosted shared-runner results remain observations, not dedicated
M-series performance guarantees. Store the actual record in bench-results;
never infer numbers from Linux or report the job as successful before observing
its result. Numeric/checker code remains frozen and its mutation evidence valid.
