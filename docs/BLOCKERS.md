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
- Hosted Linux/macOS CI observations are separate from local checks. CI jobs
  are provided; do not assert they passed before obtaining their actual results.
- Whole-process memory includes retained original JSONL and exceeds the original
  100-byte/event engineering goal. Replay-column memory is measured separately.
- Miri covers arithmetic edge paths, combined-fee rounding and allocation
  instrumentation; native high-volume properties run in full. See ADR-0006.
- Sparse I1 algebraic admission is implemented; optimized I2/I3 admission,
  streaming/mmap source storage and real engine import remain future work.

See HANDOFF_PART_01.md and the committed raw outputs for the final test,
mutation and performance evidence. No secret or account connection is required.
