# Status and remaining limitations

There is no active blocker to developing, running, checking or reducing the
synthetic Part 1 workflow in this Linux instance. The initial empty-repository
and docs.rs stop were superseded by the user's explicit instruction to create
the project and continue with full authorization.

- docs.rs remains blocked by the actual runtime destination policy. Release
  versions, dates and checksums were verified through official crates.io
  metadata/source downloads. ADR-0004 records the deviation; no proxy/TLS or
  artifact verification was bypassed. The docs.rs network addition is saved
  in the environment draft but saving is not runtime application/publication.
- Hosted Linux/macOS quality, Miri, scale/instruction and macOS benchmark jobs
  passed for 7ff05069f19358e646687e41cf5d6b80afaaec6e. Saved observations and
  workflow links are in validation-results/hosted-ci-current.json.
- Whole-process memory includes retained original JSONL and exceeds the original
  100-byte/event goal. The hosted Mac parse measurement is below the 300 MB/s
  goal with five workers on three virtual CPUs. Raw results and configuration
  are disclosed; these estimates are not silently treated as met.
- Miri covers arithmetic edge paths, combined-fee rounding and allocation
  instrumentation; native high-volume properties run in full. See ADR-0006.
- Sparse I1 algebraic admission is implemented; optimized I2/I3 admission,
  streaming/mmap source storage and real engine import remain future work.

See HANDOFF_PART_01.md and the committed raw outputs for the final test,
mutation and performance evidence. No secret or account connection is required.
