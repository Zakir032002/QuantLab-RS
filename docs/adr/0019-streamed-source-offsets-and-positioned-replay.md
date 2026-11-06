# ADR-0019: bounded batches and positioned source replay

Status: implemented; final scale/ARM and long differential gates pending.

A regular-file ingest scans fixed 64 KiB blocks to build separate u64 starts and
u32 lengths (12 bytes per line), BLAKE3 block commitments and the complete source
digest. It then reads bounded batches, default 8 MiB, parses borrowed records in
parallel, and admits them through the same builder in original source order.
A second digest accumulated over those actual parsed batches must match the
scan. No third whole-file pass or full source buffer is required.

The alternatives were mmap, a sequential-only parser, and local interners plus
column-fragment remapping. mmap is rejected: it needs unsafe code and concurrent
truncation can cause SIGBUS. A sequential-only parser leaves existing rayon
throughput unused. Borrowed record batches keep the existing parser and one
ordered builder; they need neither a second event implementation nor local-ID
remapping. One batch is in flight, bounded by chunk size or one bounded line.
Chunk sizes below one line still admit exactly that line. Small batches use the
sequential loop; larger batches use rayon. First parse errors merge in line order.

`SourceRef` and `SourceReader` provide the adapter-facing positioned-read
boundary. A fetched record is checked against its source reference and all
covering 64 KiB block digests. A changed length, truncation, terminator or block
is E-INPUT-CHANGED. Source offsets and digests live at the I/O boundary, not in
financial columns. The compatibility `source` and `lines` fields are populated
only by explicitly requested in-memory parsing; file and CLI ingestion leave
both empty. Production consumers use source_digest, source_len, line_range and
read_record. Fixtures can still explicitly own an in-memory source.

Pipes and stdin spool into a create-new owned temporary, with the input-byte
limit applied during the copy. The parsed trace owns its cleanup. `--no-spool`
rejects these inputs with E-INPUT-NOT-SEEKABLE. No mmap or product unsafe is used.
`TraceStoreBuilder` is the sole admission implementation; its bounded public
adapter entry takes canonical event JSON, so adapters cannot bypass the strict
parser, numeric policy, ordered interner or limits.

`--stats` explicitly wraps the ordinary result with retained column/interner
and source-index estimates. Process peak RSS remains null until measured
externally; vector capacity is not represented as process RSS. Ordinary report
and bundle bytes remain unchanged. File/memory equality, exact source slices,
source mutation, truncation, spooling and all nine shapes have native tests.
F03 compares every column and interner ID, source references and first errors
across sequential and randomly chunked parallel file ingestion.

Measurements are retained under evidence/part3/streaming. CP3.1's input bytes
are reused for the 1M comparison. The 10M RSS bound remains an optimization
target until its independently recorded gate passes; a successful process exit
alone is not a memory-gate pass.

The uncapped compatibility API falls back to its legacy in-memory parser only
when a record exceeds SourceRef's u32 span. Strict or explicitly bounded reads
never take that path. See ADR-0023.
