# Status and remaining limitations

There is no local blocker to developing, running, checking or reducing the
synthetic Part 1 workflow. Final Mac ARM verification has a runner-capacity
block; the verified Intel macOS retry is pending. The initial empty-repository
and docs.rs stop were superseded by the user's explicit instruction to create
the project and continue with full authorization.

- docs.rs remains blocked by the actual runtime destination policy. Release
  versions, dates and checksums were verified through official crates.io
  metadata/source downloads. ADR-0004 records the deviation; no proxy/TLS or
  artifact verification was bypassed. The docs.rs network addition is saved
  in the environment draft but saving is not runtime application/publication.
- Hosted Linux/macOS quality, Miri, scale/instruction and macOS benchmark jobs
  passed for claim-boundary source 111b1dbf62cd1b4f28fa10321adc1dc4bcf4f3da.
  Saved observations and workflow links are in
  validation-results/claim-boundaries/hosted-ci.json.
- Final source e348f56 passed Linux quality, Miri and scale/instruction checks.
  GitHub cancelled both queued ARM Mac jobs because hosted runners were not
  acquired during an arm64 capacity shortage. ADR-0009 records the standard
  macos-15-intel retry; validation-results/order-lookup/mac-cancellation.json
  retains the exact provider messages. Do not count cancellations as passes.
- Whole-process memory includes retained original JSONL and exceeds the original
  100-byte/event goal. The latest shared-Linux and earlier M1 parse observations
  are below the 300 MB/s reference goal. Raw results and configuration
  are disclosed; these estimates are not silently treated as met.
- Miri covers arithmetic edge paths, combined-fee rounding and allocation
  instrumentation; native high-volume properties run in full. See ADR-0006.
- Claim-boundary reduction uses the complete sparse checker for I1/I2/I3;
  pattern counts are incremental. Streaming/mmap source storage and real engine
  import remain future work.

See HANDOFF_CLAIM_BOUNDARIES.md for the corrections and their evidence;
HANDOFF_PART_01.md records the previous release. No secret or account connection
is required for the synthetic workflow.
