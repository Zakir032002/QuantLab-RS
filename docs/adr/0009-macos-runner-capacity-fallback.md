# ADR-0009: macOS runner capacity fallback

## Context

The final e348f56 Linux quality, Miri and scale/instruction jobs passed.
Both macOS ARM jobs were cancelled while queued: GitHub reported that the job
was not acquired by a hosted runner after multiple attempts, an internal server
error and macOS arm64 capacity constraints. This is infrastructure failure,
not a passing or failing code test. Earlier source 111b1db completed Mac quality
and actual M1 measurements; retain that record with its original provenance.

## Options

Wait/retry the same constrained ARM pool, or use a standard Intel macOS runner
for current cross-platform quality and separately labeled measurements.

## Decision

Retry quality and benchmarks with the verified standard `macos-15-intel` label.
GitHub's official runner inventory lists this x64 label:
https://github.com/actions/runner-images/blob/main/README.md .
No larger/custom runner is requested. Rust, dependencies, tests and tolerances
remain pinned. Keep the cancelled job observations and provider diagnostics.

## Consequences

New Mac numbers identify the actual Intel CPU/OS/compiler; they are not M1
estimates. The previously measured M1 record remains separately available.
The final source's M1 benchmark is not measured during this capacity outage.
Observe the retry before claiming macOS success; stop and report if runner
availability remains blocked after this retry.

## Observed outcome

All five retry jobs passed for 5e1faad, including Linux and Intel macOS quality,
Miri, scale/instructions and Intel benchmarks:
https://github.com/Zakir032002/QuantLab-RS/actions/runs/41898170782 .
The CPU is Intel Core i7-8700B with four CPUs / 14 GiB RAM, macOS 15.7.9.
Raw samples, actual compiler/machine data and a verified public transfer record
are in `bench-results/2026-11-04-lookup-macos-x86_64/`. The checkout's Rust and
manifest hashes match final production source e348f56. ARM cancellations remain
recorded separately; they are not treated as successful tests.
