# ADR-0021: remove unused and over-wide lifecycle storage

Status: measured and kept.

`Unit.first_event` had no production readers; the spine already supplies event
membership. Removing the field reduces a unit from 16 to 8 bytes without
changing unit IDs, order context or serialized outputs. Lifecycle progress is
updated only for the five order event kinds. Its eight u64 counters therefore
contained three permanently zero entries; retaining five u64 counters preserves
all count values and the existing fingerprint codec, including large counts.
Trade keys use the original u32 instrument ID and u32 interned trade ID instead
of widening the instrument to host usize. No numeric value is narrowed.

The first 10M streaming run exceeded the RSS bound. The compact-state run uses
1388056 KiB, below 140 bytes/event + 64 MiB; the 1M run uses 142824 KiB, below
180 MiB. Raw before/after commands, host/compiler metadata and outputs are in
`evidence/part3/streaming` and `evidence/part3/hot-path`. Development measurements
record worktree source hashes; they are not final cross-OS Gate S evidence.
`compact-state/rss-gates.json` derives the exact byte bounds. The seven measured
instruction workloads pass the unchanged 3% comparison (comparison.json).
The change is kept for its measured RSS improvement, not a claimed wall-time
speedup. The full checker/reducer/corpus tests remain the admission oracle.
