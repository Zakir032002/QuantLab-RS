# Part 2 checkpoint limits

CP2.2 R4 scaling gate is stopped after two measured attempts on this shared Linux
host. Raw samples: bench/results/CP2.2/dense-checker.jsonl and
 dense-checker-after-reserve.jsonl. First medians: 1,954,778 ns / 28,174,363 ns
(ratio 14.413). Reserving dense finding output changed medians to 2,206,738 ns /
26,998,455 ns (ratio 12.2346), still above the fixed 12 limit. Both 100k checks
are below 0.5 seconds. Hypothesis: output allocation/cache behavior dominates
this workload. Do not claim CP2.2 passed or change the threshold. Other Part 2
checkpoints can proceed independently. No third attempt at this gate is made.

The macOS ARM capacity limitation and older Part 1 provenance follow below.

# Status and remaining limitations

There is no local or cross-platform quality blocker to the synthetic Part 1
workflow. The standard Intel macOS retry passed. ARM runner capacity prevents
a new final-source M1 measurement; the earlier actual M1 record is retained. The initial empty-repository
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
  retains the exact provider messages. All five Intel-retry workflow jobs passed
  for 5e1faad, whose Rust/manifest hashes match e348f56. Observed results are in
  validation-results/order-lookup/hosted-ci.json; cancellations are not passes.
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
