# R1–R5 review findings

| Finding | Rebased result and evidence |
|---|---|
| R1a/b D04 400/2000 orders | Already bounded before Part 2. The CP2.0 regression passes on starting source; the old far-back-inventory root cause is refuted for this checkout. Fresh release calls/events are in CP2.12/regressions.jsonl. |
| R1c long-lived | Target the parent completed near the end of a 12053-event trace; checkpoint restores status/filled quantity. One-event suffix witness, two candidate slots. |
| R1d carried error | S6 carries a first cash error and targets the second later defect; exact target and observed opening values survive. |
| R2 uncovered fills | Existing tests cover cash and per-instrument tails. CP2.0 fixes precedence so an earlier proven violation wins while uncovered diagnostics remain. Full-trace coverage remains required when no violation exists. |
| R3 I2 evidence | Existing local I2 is retained. CP2.0 removes global claim-frontier coupling; tests cover no cash claims and no unrelated assets. The funding fixture retains another asset only when the target's buy requires its cash. |
| R4 checker cost | Existing pattern counts were already incremental. Remove dynamic iterator dispatch, prepare order limits once, reserve dense finding output. Absolute time passes; 10k-to-100k ratio 12.2346 misses 12. Both attempts and the executable failing ratchet remain committed. Checkpoint stopped, threshold unchanged. |
| R5 helper/fingerprint mutations | Direct helper tests cover accepted batch selection, caps, dependent removals, pins and no-ops. Forced hash collision and fingerprint encoding tests remain active. Final campaign outcomes and separate floors are recorded in MUTANTS_PART_02.md. |

The public-bounds cache suspicion during the audit was refuted, rather than
reported as a discovered bug. The final test checks the actual normalization
contract and a defensive private-batch precondition. See ADR-0015 and the raw
incorrect-hypothesis record. No existing test was marked ignored to manufacture
Stage A failures. The explicitly authorized req_check_011 correction is in
ADR-0010; no golden, catalogue, rounding or tolerance was weakened.
