# ADR-0016: streaming generation and atomic files

Context: existing generators accumulate their complete output in a Vec. The
same lifecycle logic can write any std::io::Write without a second generator.

Decision: retain generate/generate_rich as Vec wrappers; expose generate_into
and generate_rich_into returning (target sequence, emitted event count). Reuse
the existing shape configuration. The event counter supplies metadata directly,
without rescanning output for newlines. No existing generator bytes change.

The qlrs-gen entry point uses SplitMix64-seeded xoshiro256**, with frozen vectors
computed independently in Python. Legacy seed behaviour is unchanged; only the
new entry point uses the new stream. A 1 MiB BufWriter sits outside the digest
writer so BLAKE3 sees full buffered blocks rather than tiny formatting writes.

New presets support simple/interleaved/long-lived/sparse and S1–S9 scheduling.
Simple presets emit the exact advertised event count. Rich profiles record
actual counts, including bootstrap/claim/partial-fill overhead. S9 keeps its
staggered schedules; the old small-fixture-only final-claim removal and NOISE
injection remain in generate_shape, not the new scale entry point.

Atomic writes use a create_new sibling temporary, writer flush, file sync,
rename and parent-directory sync. Cleanup only owns a temporary successfully
created by this invocation. Trace and metadata files are individually atomic;
they are not advertised as a two-file transaction. Shared atomic_file will also
serve report/bundle writing. No new unsafe code or dependency versions are added.

Provenance: source revision reuses the bundle's existing embedded build revision.
Worktree measurements record that base revision plus a source patch and SHA256s;
they are not mislabeled as measurements of an already-committed source tree.
Raw Linux memory and repeat-digest evidence is in evidence/part3/gen. ARM
validation remains pending; no cross-OS pass or Gate S is claimed yet.
