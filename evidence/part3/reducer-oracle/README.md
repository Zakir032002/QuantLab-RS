# P3-ORACLE-001

Discovered by the new structured target's stable seed replay, then reproduced
and minimized with the real reduce_sound libFuzzer target. This is an oracle
request error, not a demonstrated product arithmetic/reducer defect.

The 13-byte input decodes to seed 3, S8, 23 orders, D08, selection bits zero.
The fixture's reported later anchor at seq 72 has both event-after-terminal and
overfill. SPEC REQ-REDUCE-007 intentionally refuses another lifecycle breach
alongside the selected target. The reducer's default earliest violation at seq
65 is independently reducible and preserves the exact InstanceKey and class_fp;
see diagnosis.txt. No product verdict or fingerprint code changes are needed.

The first commit keeps this minimized input and an ignored failing oracle test.
A separate correction will use the public default target rather than treating a
generator's later mutation anchor as a promise of an isolatable witness. The
same soundness, exact-instance, class-fingerprint and byte assertions stay.

Minimization: cargo +nightly-2026-11-04 fuzz tmin --fuzz-dir fuzz --release -s none
reduce_sound /tmp/qlrs-reduce-seed -- -max_total_time=60 -rss_limit_mb=2048
-timeout=10 -print_final_stats=1. Raw tool output is tmin.txt. This is not counted
as a long campaign and does not authorize an invariant change.
