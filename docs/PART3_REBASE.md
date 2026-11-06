# Part 3 rebase

Start: 9b9c335, with unchanged product source from 766632a. The user explicitly
authorized reopening R4 and continuing Part 3 after the initial CP3.0 stop.
Historical failed measurements are retained. A new run on unchanged source
passes the original limits; see evidence/part3/r4-reopened/before. This is a
wall-time revalidation, not a claimed code optimization.

| Instructions' name/assumption | Current contract |
|---|---|
| 60 tests | 82 tests; floor 82 |
| Numeric/checker 347/387 | 359/399; reducer 190/270; fingerprints 11/11, separate floors |
| Next ADR after Part 2 | 0016; 0001–0015 are occupied |
| Reducer | reduce_with_options; reduce_with_limits compatibility entry; SBR default, ddmin-closures reference |
| Limits | ReductionLimits; preserve --time-limit-ms and --max-calls |
| Dense/selected checker | Checker::check / check_selected; check_witness / check_witness_selected provide scoped evidence semantics |
| Checkpoint | schema_version 1; orders/trades, independent expected state and observed opening windows |
| Bundles | v1 writer, v0 reader; verify_with_options and --recheck-certificate |
| Generator | generate, generate_rich and generate_shape; S1–S9; all existing bytes preserved |
| Defects D01–D14 | D01–D11 violate; D12/D13 unsupported; D14 inconclusive. Keep the protected catalogue, including these oracle outcomes. |
| Algebraic evaluator | ContributionTable / FastPredicate; checked residual hints, not model/lifecycle admission; None requires full oracle |
| Fingerprint | qlrs-fp1 class bytes unchanged; InstanceKey is a separate exact identity |
| Input builder | Private Builder in qlrs-io; promote/reuse it rather than creating a second ingestion implementation |
| Memory accounting | TraceStore::heap_bytes already sums retained column allocations; reuse as the statistics implementation |
| Machine | Current cloud Linux x86_64; Apple Silicon measurements require a real ARM runner or the user's Mac |
| Overflow | Well-formed numeric overflow INCONCLUSIVE; malformed decimal input error; excess scale/model limits UNSUPPORTED |
| Evidence precedence | Unsupported retains priority; a proven violation outranks missing-evidence diagnostics; full traces require coverage, witnesses only target scope |

Fuzz oracles must use these existing domains: S7's deliberately erroneous
observations are not clean input; D12–D14 are not violating operators; algebraic
residual agreement does not assert lifecycle admissibility. Preserve the full
admission oracle and compare residuals where the numerical evaluator is defined.
No monotonicity assumption is introduced for the reducer or unsupported tails.

The initial CP3.0 blocked-report commit remains in history. The resumed checkpoint
records fresh results rather than rewriting that report or backdating a pass.
