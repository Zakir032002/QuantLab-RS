//! Strict JSONL boundary, borrowed parallel parsing and verbatim source writing.
#![forbid(unsafe_code)]
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
mod limits;
#[cfg(kani)]
mod proofs;
mod source;
pub use limits::{Limits, SCHEMA_KEYS, validate_line};
use qlrs_num::{Fixed, NumericError};
use qlrs_schema::{Account, AliveMask, Header, Issue, Kind, NONE, Side, Status, TraceStore, Unit};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
pub use source::{SourceReader, SourceRef};
use std::borrow::Cow;
use std::{collections::HashMap, error::Error, fmt, fs, io::Write, path::Path};

/// Stable distinction between malformed input, model capacity and budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorKind {
    /// Malformed input or a failed file operation.
    #[serde(rename = "INPUT_ERROR")]
    Input,
    /// A well-formed input exceeds model representation limits.
    Unsupported,
    /// A user resource budget was exhausted.
    #[serde(rename = "INCONCLUSIVE")]
    Budget,
}
impl ErrorKind {
    /// Fixed CLI exit code for this error category.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Input => 1,
            Self::Unsupported => 4,
            Self::Budget => 3,
        }
    }
}
/// A strict boundary error with its one-based source line.
#[derive(Debug, Clone, Serialize)]
pub struct InputError {
    /// Typed status category.
    #[serde(rename = "status")]
    pub kind: ErrorKind,
    /// Stable error identifier.
    pub code: &'static str,
    /// Source line; zero denotes a file I/O error.
    pub line: usize,
    /// Human-readable cause.
    pub message: String,
}
impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: line {}: {}", self.code, self.line, self.message)
    }
}
impl Error for InputError {}
fn err(line: usize, message: impl Into<String>) -> InputError {
    let message = message.into();
    let code = if message.starts_with("duplicate field") {
        "E-DUPLICATE-KEY"
    } else if message.starts_with("unknown field") {
        "E-UNKNOWN-KEY"
    } else if message.contains("utf-8")
        || message.contains("unicode")
        || message.contains("surrogate")
        || message.contains("hex escape")
    {
        "E-UTF8"
    } else if line == 0 {
        "E-IO"
    } else {
        "E-JSON"
    };
    InputError {
        kind: ErrorKind::Input,
        code,
        line,
        message,
    }
}

/// Decode JSON outside the core crates.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, InputError> {
    serde_json::from_slice(bytes).map_err(|e| err(e.line(), e.to_string()))
}
/// Canonical compact JSON, newline terminated, with fixed struct field order.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, InputError> {
    let mut out = serde_json::to_vec(value).map_err(|e| err(0, e.to_string()))?;
    out.push(b'\n');
    Ok(out)
}
/// BLAKE3 content digest in lower-case hexadecimal.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Write and sync a sibling temporary file, then atomically replace the target.
/// A failed writer leaves the previous target intact and removes its temporary.
pub fn atomic_file<T>(
    path: impl AsRef<Path>,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<T>,
) -> std::io::Result<T> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = path.as_ref();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = parent.join(format!(
        ".qlrs.tmp.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        let value = write(&mut file)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(value)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Record<'a> {
    OrderSubmitted {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        client_order_id: Cow<'a, str>,
        #[serde(borrow)]
        instrument_id: Cow<'a, str>,
        #[serde(borrow)]
        side: Cow<'a, str>,
        #[serde(borrow)]
        order_type: Cow<'a, str>,
        #[serde(borrow)]
        quantity: Cow<'a, str>,
    },
    OrderAccepted {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        client_order_id: Cow<'a, str>,
        #[serde(borrow)]
        venue_order_id: Cow<'a, str>,
    },
    OrderRejected {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        client_order_id: Cow<'a, str>,
        #[serde(borrow)]
        reason: Cow<'a, str>,
    },
    OrderFilled {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        client_order_id: Cow<'a, str>,
        #[serde(borrow)]
        trade_id: Cow<'a, str>,
        #[serde(borrow)]
        last_qty: Cow<'a, str>,
        #[serde(borrow)]
        last_px: Cow<'a, str>,
        #[serde(borrow)]
        commission: Cow<'a, str>,
        #[serde(borrow)]
        commission_currency: Cow<'a, str>,
        #[serde(borrow)]
        liquidity_side: Cow<'a, str>,
    },
    OrderCanceled {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        client_order_id: Cow<'a, str>,
    },
    AccountClaim {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        currency: Cow<'a, str>,
        #[serde(borrow)]
        balance_total: Cow<'a, str>,
    },
    PositionClaim {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        instrument_id: Cow<'a, str>,
        #[serde(borrow)]
        net_qty: Cow<'a, str>,
    },
    Unsupported {
        seq: u64,
        ts_event: u64,
        #[serde(borrow)]
        account_id: Cow<'a, str>,
        #[serde(borrow)]
        provenance: Cow<'a, str>,
        #[serde(borrow)]
        feature: Cow<'a, str>,
    },
}
#[allow(clippy::items_after_statements)]
fn parse_record(bytes: &[u8], line: usize) -> Result<Record<'_>, InputError> {
    match serde_json::from_slice::<Record<'_>>(bytes) {
        Ok(record) => Ok(record),
        Err(original) => {
            #[derive(Deserialize)]
            struct Tag<'a> {
                #[serde(rename = "type")]
                #[serde(borrow)]
                kind: Cow<'a, str>,
            }
            let tag: Tag<'_> =
                serde_json::from_slice(bytes).map_err(|e| err(line, e.to_string()))?;
            if matches!(
                tag.kind.as_ref(),
                "header"
                    | "OrderSubmitted"
                    | "OrderAccepted"
                    | "OrderRejected"
                    | "OrderFilled"
                    | "OrderCanceled"
                    | "AccountClaim"
                    | "PositionClaim"
                    | "Unsupported"
            ) {
                return Err(err(line, original.to_string()));
            }
            #[derive(Deserialize)]
            struct Common<'a> {
                seq: u64,
                ts_event: u64,
                #[serde(borrow)]
                account_id: Cow<'a, str>,
                #[serde(borrow)]
                provenance: Cow<'a, str>,
            }
            let common: Common<'_> =
                serde_json::from_slice(bytes).map_err(|e| err(line, e.to_string()))?;
            Ok(Record::Unsupported {
                seq: common.seq,
                ts_event: common.ts_event,
                account_id: common.account_id,
                provenance: common.provenance,
                feature: tag.kind,
            })
        }
    }
}
/// Canonicalize one bounded JSON record for adapter/fuzz round-trip checks.
/// Unknown event kinds normalize to their existing Unsupported representation.
pub fn canonical_record(bytes: &[u8], limits: Limits) -> Result<Vec<u8>, InputError> {
    #[derive(Deserialize)]
    struct Tag<'a> {
        #[serde(rename = "type", borrow)]
        kind: Cow<'a, str>,
    }
    limits.validate()?;
    validate_line(bytes, 1, limits)?;
    let tag: Tag<'_> = serde_json::from_slice(bytes).map_err(|e| err(1, e.to_string()))?;
    if tag.kind == "header" {
        let header: Header = decode(bytes)?;
        if header.schema != "qlrs-trace/v0" {
            return Err(err(1, "expected header schema qlrs-trace/v0"));
        }
        encode(&header)
    } else {
        encode(&parse_record(bytes, 1)?)
    }
}
impl Record<'_> {
    fn common(&self) -> (u64, u64, &str, &str) {
        match self {
            Self::OrderSubmitted {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::OrderAccepted {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::OrderRejected {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::OrderFilled {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::OrderCanceled {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::AccountClaim {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::PositionClaim {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            }
            | Self::Unsupported {
                seq,
                ts_event,
                account_id,
                provenance,
                ..
            } => (*seq, *ts_event, account_id, provenance),
        }
    }
}

/// Retained heap estimates; process peak RSS is measured externally.
#[derive(Debug, Serialize)]
pub struct TraceStats {
    /// Parsed event count.
    pub events: usize,
    /// Retained replay column capacities.
    pub column_bytes: usize,
    /// Column bytes per event (integer floor).
    pub bytes_per_event: usize,
    /// Retained source offset capacities.
    pub offsets_bytes: usize,
    /// Distinct interned strings.
    pub interner_entries: usize,
    /// Interner arena and index capacities.
    pub interner_bytes: usize,
    /// Columns, interner and source bookkeeping; not allocator/process overhead.
    pub heap_bytes_estimate: usize,
    /// Null unless measured externally; no fabricated in-process peak sample.
    pub peak_rss: Option<u64>,
}

/// Original bytes are isolated at the I/O boundary, not retained in replay columns.
#[derive(Debug)]
pub struct ParsedTrace {
    /// Compact replay/check/reduction store.
    pub store: TraceStore,
    /// Original bytes for explicit in-memory parsing only. File ingest leaves
    /// this empty; consumers use `source_digest`/`source_len`/`read_record` instead.
    pub source: Vec<u8>,
    file: Option<source::FileSource>,
    /// One range for the header, followed by one per event; includes newline.
    pub lines: Vec<std::ops::Range<usize>>,
}
impl ParsedTrace {
    /// Read a trace using safe buffered file I/O.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, InputError> {
        let path = path.as_ref();
        match Self::read_chunked(path, None, 8 << 20) {
            Err(error)
                if error.code == "U-LIMIT-INDEX" && error.message == "line length exceeds u32" =>
            {
                // Legacy uncapped input may exceed SourceRef's new u32 span.
                // Bounded callers never take this compatibility path.
                Self::parse(fs::read(path).map_err(|e| err(0, e.to_string()))?)
            }
            result => result,
        }
    }
    /// Strictly parse a complete UTF-8 JSONL buffer.
    pub fn parse(source: Vec<u8>) -> Result<Self, InputError> {
        Self::parse_mode(source, None)
    }
    fn parse_mode(source: Vec<u8>, limits: Option<Limits>) -> Result<Self, InputError> {
        if let Some(limits) = limits {
            limits.validate()?;
            if source.len() as u64 > limits.max_input_bytes {
                return Err(limits::error(
                    0,
                    "E-LIMIT-INPUT-BYTES",
                    "input exceeds byte limit",
                ));
            }
        }
        if source.starts_with(&[0xef, 0xbb, 0xbf]) {
            return Err(err(1, "UTF-8 BOM is forbidden"));
        }
        if source.last() != Some(&b'\n') {
            return Err(err(1, "trace must end with a newline"));
        }
        if let Err(error) = std::str::from_utf8(&source) {
            let line = source
                .iter()
                .take(error.valid_up_to())
                .filter(|byte| **byte == b'\n')
                .count()
                .saturating_add(1);
            return Err(err(line, error.to_string()));
        }
        let mut lines = Vec::new();
        let mut start = 0;
        for (end, byte) in source.iter().enumerate() {
            if *byte == b'\n' {
                if limits.is_some_and(|l| end + 1 - start > l.max_line_bytes) {
                    return Err(limits::error(
                        lines.len() + 1,
                        "E-LIMIT-LINE-BYTES",
                        "line exceeds byte limit",
                    ));
                }
                lines.push(start..end + 1);
                start = end + 1;
            }
        }
        if let Some(limits) = limits {
            if lines.len().saturating_sub(1) > limits.max_events {
                return Err(limits::error(
                    limits.max_events + 2,
                    "U-LIMIT-EVENTS",
                    "event limit exceeded",
                ));
            }
            let estimate = (source.len() as u64)
                .saturating_mul(2)
                .saturating_add((lines.len() as u64).saturating_mul(160))
                .saturating_add(64 << 20);
            if limits
                .memory_budget_bytes
                .is_some_and(|budget| estimate > budget)
            {
                return Err(limits::error(
                    0,
                    "I-BUDGET-MEMORY",
                    "pre-scan allocation estimate exceeds budget",
                ));
            }
        }
        let first = lines.first().ok_or_else(|| err(1, "missing header"))?;
        if let Some(limits) = limits {
            validate_line(
                source
                    .get(first.clone())
                    .ok_or_else(|| err(1, "missing header"))?,
                1,
                limits,
            )?;
        }
        let header: Header = serde_json::from_slice(
            source
                .get(first.clone())
                .ok_or_else(|| err(1, "missing header"))?,
        )
        .map_err(|e| err(1, e.to_string()))?;
        let mut builder = TraceStoreBuilder::new(header)?;
        builder.limits = limits;
        // Bounded parallel batches borrow the immutable source; merge in source order.
        for (batch_index, batch) in lines.get(1..).unwrap_or_default().chunks(8192).enumerate() {
            let parse = |(offset, range): (usize, &std::ops::Range<usize>)| {
                let line = batch_index * 8192 + offset + 2;
                let bytes = source
                    .get(range.clone())
                    .ok_or_else(|| err(line, "invalid source range"))?;
                if let Some(limits) = limits {
                    validate_line(bytes, line, limits)?;
                }
                parse_record(bytes, line)
            };
            // Small traces avoid thread-pool startup; large batches remain parallel.
            let records: Vec<_> = if batch.len() < 1024 {
                batch.iter().enumerate().map(parse).collect()
            } else {
                batch.par_iter().enumerate().map(parse).collect()
            };
            for (offset, record) in records.into_iter().enumerate() {
                builder.push(record?, batch_index * 8192 + offset + 2)?;
            }
        }
        builder.store.compact();
        Ok(Self {
            store: builder.store,
            source,
            lines,
            file: None,
        })
    }
    /// Parse with explicitly selected resource limits. Existing parse defaults
    /// remain unchanged pending the recorded R9 review.
    pub fn parse_with_limits(source: Vec<u8>, limits: Limits) -> Result<Self, InputError> {
        Self::parse_mode(source, Some(limits))
    }
    /// Read a regular file with line bounds enforced before whole-line buffering.
    pub fn read_with_limits(path: impl AsRef<Path>, limits: Limits) -> Result<Self, InputError> {
        Self::read_chunked(path, Some(limits), 8 << 20)
    }
    /// Stream bounded batches and merge parsed records in source order. A chunk
    /// smaller than one line still reads just that line; no line is split into
    /// independent JSON records. The sequential path is selected at one byte.
    pub fn read_chunked(
        path: impl AsRef<Path>,
        limits: Option<Limits>,
        chunk_bytes: usize,
    ) -> Result<Self, InputError> {
        use std::io::{Read, Seek, SeekFrom};
        if let Some(limits) = limits {
            limits.validate()?;
        }
        if !(1..=16 << 20).contains(&chunk_bytes) {
            return Err(limits::error(
                0,
                "E-LIMIT-CONFIG",
                "chunk size must be 1..16 MiB",
            ));
        }
        let file = fs::File::open(path).map_err(|e| err(0, e.to_string()))?;
        if !file
            .metadata()
            .map_err(|e| err(0, e.to_string()))?
            .is_file()
        {
            return Err(limits::error(
                0,
                "E-INPUT-NOT-SEEKABLE",
                "input is not a regular file",
            ));
        }
        let mut source = source::FileSource::scan(file, limits)?;
        let mut buffer = Vec::new();
        let mut first = 0;
        let mut ingestion_digest = blake3::Hasher::new();
        let mut builder: Option<TraceStoreBuilder> = None;
        while first < source.starts.len() {
            let start = *source
                .starts
                .get(first)
                .ok_or_else(|| err(first + 1, "missing offset"))?;
            let mut stop = first;
            let mut bytes = 0_usize;
            while let Some(length) = source.lengths.get(stop) {
                let next = bytes
                    .checked_add(*length as usize)
                    .ok_or_else(|| err(stop + 1, "chunk size overflow"))?;
                if stop > first && next > chunk_bytes {
                    break;
                }
                bytes = next;
                stop += 1;
            }
            buffer.resize(bytes, 0);
            source
                .file
                .seek(SeekFrom::Start(start))
                .and_then(|_| source.file.read_exact(&mut buffer))
                .map_err(|e| err(first + 1, e.to_string()))?;
            ingestion_digest.update(&buffer);
            if first == 0 {
                let length = *source
                    .lengths
                    .first()
                    .ok_or_else(|| err(1, "missing header"))? as usize;
                let bytes = buffer
                    .get(..length)
                    .ok_or_else(|| err(1, "missing header"))?;
                if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                    return Err(err(1, "UTF-8 BOM is forbidden"));
                }
                if let Some(limits) = limits {
                    validate_line(bytes, 1, limits)?;
                }
                let header: Header =
                    serde_json::from_slice(bytes).map_err(|e| err(1, e.to_string()))?;
                let mut value = TraceStoreBuilder::new(header)?;
                value.limits = limits;
                builder = Some(value);
            }
            let skip = usize::from(first == 0);
            let mut ranges = Vec::with_capacity(stop.saturating_sub(first));
            let mut offset = 0_usize;
            for line in first..stop {
                let length = *source
                    .lengths
                    .get(line)
                    .ok_or_else(|| err(line + 1, "missing length"))?
                    as usize;
                if line >= first + skip {
                    ranges.push((line + 1, offset..offset + length));
                }
                offset += length;
            }
            let parse = |(line, range): &(usize, std::ops::Range<usize>)| {
                let bytes = buffer
                    .get(range.clone())
                    .ok_or_else(|| err(*line, "invalid range"))?;
                if let Some(limits) = limits {
                    validate_line(bytes, *line, limits)?;
                }
                std::str::from_utf8(bytes).map_err(|e| err(*line, e.to_string()))?;
                parse_record(bytes, *line)
            };
            let records: Vec<_> = if ranges.len() < 32 || chunk_bytes == 1 {
                ranges.iter().map(parse).collect()
            } else {
                ranges.par_iter().map(parse).collect()
            };
            let builder = builder.as_mut().ok_or_else(|| err(1, "missing header"))?;
            for ((line, _), record) in ranges.iter().zip(records) {
                builder.push(record?, *line)?;
            }
            first = stop;
        }
        if ingestion_digest.finalize().to_hex().as_str() != source.digest
            || source
                .file
                .metadata()
                .map_err(|e| err(0, e.to_string()))?
                .len()
                != source.bytes
        {
            return Err(limits::error(
                0,
                "E-INPUT-CHANGED",
                "source changed during parse",
            ));
        }
        let mut builder = builder.ok_or_else(|| err(1, "missing header"))?;
        builder.store.compact();
        Ok(Self {
            store: builder.store,
            source: Vec::new(),
            lines: Vec::new(),
            file: Some(source),
        })
    }
    /// Copy a non-seekable source into an owned temporary, enforcing the input
    /// byte limit during the copy. The temporary lives exactly as long as the trace.
    pub fn spool(
        mut reader: impl std::io::Read,
        limits: Option<Limits>,
        chunk_bytes: usize,
    ) -> Result<Self, InputError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "qlrs-spool-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| err(0, e.to_string()))?;
        let result = (|| {
            let mut buffer = vec![0_u8; 64 << 10];
            let mut total = 0_u64;
            loop {
                let count = reader
                    .read(&mut buffer)
                    .map_err(|e| err(0, e.to_string()))?;
                if count == 0 {
                    break;
                }
                total = total.checked_add(count as u64).ok_or_else(|| {
                    limits::error(0, "E-LIMIT-INPUT-BYTES", "input byte count overflow")
                })?;
                if limits.is_some_and(|l| total > l.max_input_bytes) {
                    return Err(limits::error(
                        0,
                        "E-LIMIT-INPUT-BYTES",
                        "input exceeds byte limit",
                    ));
                }
                file.write_all(
                    buffer
                        .get(..count)
                        .ok_or_else(|| err(0, "invalid copy length"))?,
                )
                .map_err(|e| err(0, e.to_string()))?;
            }
            file.sync_all().map_err(|e| err(0, e.to_string()))?;
            let mut trace = Self::read_chunked(&path, limits, chunk_bytes)?;
            trace
                .file
                .as_mut()
                .ok_or_else(|| err(0, "missing spooled source"))?
                .remove_on_drop(path.clone());
            Ok(trace)
        })();
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
    /// Resource statistics without sampling a process-global allocator.
    #[must_use]
    pub fn stats(&self) -> TraceStats {
        let columns = self.store.column_bytes();
        TraceStats {
            events: self.store.len(),
            column_bytes: columns,
            bytes_per_event: columns.checked_div(self.store.len()).unwrap_or(0),
            offsets_bytes: self.file.as_ref().map_or(
                self.lines.capacity() * std::mem::size_of::<std::ops::Range<usize>>(),
                |f| f.starts.capacity() * 8 + f.lengths.capacity() * 4,
            ),
            interner_entries: self.store.strings.len(),
            interner_bytes: self.store.strings.heap_bytes(),
            heap_bytes_estimate: columns.saturating_add(self.source_heap_bytes()),
            peak_rss: None,
        }
    }
    /// Original input commitment, independent of storage backend.
    #[must_use]
    pub fn source_digest(&self) -> String {
        self.file
            .as_ref()
            .map_or_else(|| digest(&self.source), |f| f.digest.clone())
    }
    /// Original source length, without fetching its bytes.
    #[must_use]
    pub fn source_len(&self) -> u64 {
        self.file
            .as_ref()
            .map_or(self.source.len() as u64, |f| f.bytes)
    }
    /// Number of source lines, including the header.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.file
            .as_ref()
            .map_or(self.lines.len(), |f| f.starts.len())
    }
    /// Original byte range for a source line, including its newline.
    #[must_use]
    pub fn line_range(&self, line: usize) -> Option<std::ops::Range<usize>> {
        if let Some(file) = &self.file {
            let reference = file.reference(line)?;
            let start = usize::try_from(reference.offset).ok()?;
            Some(start..start.checked_add(reference.len as usize)?)
        } else {
            self.lines.get(line).cloned()
        }
    }
    /// Source location for an event (the header is source line zero).
    #[must_use]
    pub fn source_ref(&self, event: usize) -> Option<SourceRef> {
        self.line_ref(event.checked_add(1)?)
    }
    fn line_ref(&self, line: usize) -> Option<SourceRef> {
        if let Some(file) = &self.file {
            file.reference(line)
        } else {
            let range = self.lines.get(line)?;
            Some(SourceRef {
                file_id: 0,
                offset: u64::try_from(range.start).ok()?,
                len: u32::try_from(range.len()).ok()?,
                line_or_record: u64::try_from(line).ok()?.checked_add(1)?,
            })
        }
    }
    /// Offset and source-commitment allocations; excludes replay columns.
    #[must_use]
    pub fn source_heap_bytes(&self) -> usize {
        self.file.as_ref().map_or(
            self.source.capacity()
                + self.lines.capacity() * std::mem::size_of::<std::ops::Range<usize>>(),
            source::FileSource::heap_bytes,
        )
    }
    /// Write original header and kept observed lines, verifying source blocks.
    pub fn write_kept(&self, alive: &AliveMask, mut writer: impl Write) -> Result<(), InputError> {
        for line in 0..self.line_count() {
            if line == 0 || alive.contains_event(&self.store, line - 1) {
                if let Some(range) = self.lines.get(line) {
                    writer
                        .write_all(
                            self.source
                                .get(range.clone())
                                .ok_or_else(|| err(line + 1, "missing source range"))?,
                        )
                        .map_err(|e| err(0, e.to_string()))?;
                } else {
                    let reference = self
                        .line_ref(line)
                        .ok_or_else(|| err(line + 1, "missing source reference"))?;
                    writer
                        .write_all(&self.read_record(&reference)?)
                        .map_err(|e| err(0, e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}
impl SourceReader for ParsedTrace {
    fn read_record(&self, reference: &SourceRef) -> Result<Vec<u8>, InputError> {
        if let Some(file) = &self.file {
            return file.read(reference);
        }
        let line = usize::try_from(reference.line_or_record)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| err(0, "invalid source reference"))?;
        if self.line_ref(line).as_ref() != Some(reference) {
            return Err(err(line + 1, "invalid source reference"));
        }
        let range = self
            .lines
            .get(line)
            .ok_or_else(|| err(line + 1, "missing source range"))?;
        self.source
            .get(range.clone())
            .map(<[u8]>::to_vec)
            .ok_or_else(|| err(line + 1, "invalid source range"))
    }
}

fn canonical_decimal(text: &str, value: Fixed) -> bool {
    if value.raw() == 0 && text.starts_with('-') {
        return false;
    }
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let integer = unsigned.split('.').next().unwrap_or("");
    !(integer.len() > 1 && integer.starts_with('0'))
}
fn negative_decimal(text: &str) -> bool {
    text.starts_with('-')
        && text
            .bytes()
            .any(|byte| byte.is_ascii_digit() && byte != b'0')
}
fn decimal_syntax(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(i, f)| (i, Some(f)));
    !integer.is_empty()
        && integer.bytes().all(|b| b.is_ascii_digit())
        && fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}
/// One deterministic, resource-bounded builder for JSONL and adapter events.
pub struct TraceStoreBuilder {
    store: TraceStore,
    orders: Vec<u32>,
    instruments: HashMap<String, u32, foldhash::fast::FixedState>,
    limits: Option<Limits>,
}
impl TraceStoreBuilder {
    /// Start from the same strict header used by JSONL ingestion.
    pub fn with_limits(header: Header, limits: Limits) -> Result<Self, InputError> {
        limits.validate()?;
        validate_line(&encode(&header)?, 1, limits)?;
        let mut builder = Self::new(header)?;
        builder.limits = Some(limits);
        Ok(builder)
    }
    /// Admit one canonical adapter event in increasing original sequence order.
    pub fn push_json(&mut self, bytes: &[u8], line: usize) -> Result<(), InputError> {
        if let Some(limits) = self.limits {
            validate_line(bytes, line, limits)?;
        }
        self.push(parse_record(bytes, line)?, line)
    }
    /// Finish and compact the shared replay store.
    #[must_use]
    pub fn finish(mut self) -> TraceStore {
        self.store.compact();
        self.store
    }
    fn new(header: Header) -> Result<Self, InputError> {
        if header.record_type != "header" || header.schema != "qlrs-trace/v0" {
            return Err(err(1, "expected header schema qlrs-trace/v0"));
        }
        if header.accounts.is_empty() || header.currencies.is_empty() {
            return Err(err(1, "accounts and currencies cannot be empty"));
        }

        let mut instruments = HashMap::default();
        for (index, instrument) in header.instruments.iter().enumerate() {
            if instruments
                .insert(
                    instrument.id.clone(),
                    qlrs_schema::checked_index(index)
                        .map_err(|_| err(1, "too many instruments"))?,
                )
                .is_some()
            {
                return Err(err(1, "duplicate instrument id"));
            }
        }
        let mut result = Self {
            store: TraceStore::new(header),
            orders: Vec::new(),
            instruments,
            limits: None,
        };
        let h = &result.store.header;
        if h.accounts.len() != 1
            || h.currencies.len() != 1
            || h.model.account_type != "CASH"
            || h.currencies.first().is_none_or(|c| {
                c.code != h.model.base_currency
                    || !(3..=8).contains(&c.code.len())
                    || c.precision > 18
            })
            || h.model.commission_max_scale.is_some_and(|s| s > 18)
            || h.instruments.iter().any(|i| {
                i.quote_currency != h.model.base_currency
                    || i.qty_precision > 18
                    || i.px_precision > 18
            })
        {
            result.issue(Status::Unsupported, None, "model-outside-part-1");
        }
        let account = result
            .store
            .header
            .accounts
            .first()
            .cloned()
            .unwrap_or(Account {
                account_id: String::new(),
                starting_balance: String::new(),
            });
        let precision = result
            .store
            .header
            .currencies
            .first()
            .map_or(18, |c| c.precision.min(18));
        let _ = result.number(&account.starting_balance, precision, None, 1)?;
        Ok(result)
    }
    fn issue(&mut self, status: Status, seq: Option<u64>, reason: &str) {
        self.store.issues.push(Issue {
            status,
            seq,
            reason: reason.into(),
        });
    }
    fn intern(&mut self, text: &str, line: usize) -> Result<u32, InputError> {
        if let Some(limits) = self.limits {
            limits.string(text, line)?;
            if self.store.strings.len() >= limits.max_interned
                && self.store.strings.lookup(text).is_none()
            {
                return Err(limits::error(
                    line,
                    "U-LIMIT-INTERNED",
                    "interner limit exceeded",
                ));
            }
        }
        self.store.strings.intern(text).map_err(|e| err(line, e))
    }
    fn number(
        &mut self,
        text: &str,
        precision: u8,
        seq: Option<u64>,
        line: usize,
    ) -> Result<Fixed, InputError> {
        if let Some(limits) = self.limits {
            limits.number(text, line)?;
        }
        let parsed = Fixed::parse(text, precision.min(18));
        if parsed.is_err() && !decimal_syntax(text) {
            return Err(err(line, "invalid decimal syntax"));
        }
        if parsed.is_err()
            && text
                .split_once('.')
                .is_some_and(|(_, fraction)| fraction.len() > usize::from(precision.min(18)))
        {
            self.issue(Status::Unsupported, seq, "unsupported-scale");
            return Fixed::new(0, 0).map_err(|e| err(line, e.to_string()));
        }
        match parsed {
            Ok(value) => Ok(value),
            Err(NumericError::InvalidDecimal) => {
                Err(err(line, format!("invalid decimal string {text:?}")))
            }
            Err(NumericError::UnsupportedScale) => {
                self.issue(Status::Unsupported, seq, "unsupported-scale");
                Fixed::new(0, 0).map_err(|e| err(line, e.to_string()))
            }
            Err(_) => {
                self.issue(Status::Inconclusive, seq, "numeric-overflow");
                Fixed::new(0, 0).map_err(|e| err(line, e.to_string()))
            }
        }
    }
    fn instrument(&mut self, text: &str, seq: u64) -> u32 {
        if let Some(id) = self.instruments.get(text) {
            *id
        } else {
            self.issue(Status::Unsupported, Some(seq), "unknown-instrument");
            NONE
        }
    }
    fn new_unit(&mut self, order: Option<u32>, line: usize) -> Result<u32, InputError> {
        let id = qlrs_schema::checked_index(self.store.units.len())
            .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
        self.store.units.push(Unit { order });
        Ok(id)
    }
    fn order(&mut self, text: &str, line: usize) -> Result<u32, InputError> {
        let symbol = self.intern(text, line)?;
        let slot = symbol as usize;
        if let Some(id) = self.orders.get(slot).copied().filter(|id| *id != NONE) {
            return Ok(id);
        }
        let id = qlrs_schema::checked_index(self.store.orders.id.len())
            .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
        let unit = self.new_unit(Some(id), line)?;
        if slot >= self.orders.len() {
            self.orders.resize(
                slot.checked_add(1)
                    .ok_or_else(|| limits::error(line, "U-LIMIT-INDEX", "order index overflow"))?,
                NONE,
            );
        }
        *self
            .orders
            .get_mut(slot)
            .ok_or_else(|| err(line, "missing order slot"))? = id;
        let o = &mut self.store.orders;
        o.id.push(symbol);
        o.instrument.push(NONE);
        o.side.push(Side::Buy);
        o.qty_raw.push(0);
        o.qty_scale.push(0);
        o.unit.push(unit);
        Ok(id)
    }
    // Consume the boundary record so it cannot escape the merge batch.
    #[allow(clippy::needless_pass_by_value)]
    fn push(&mut self, record: Record<'_>, line: usize) -> Result<(), InputError> {
        if self
            .limits
            .is_some_and(|limits| self.store.len() >= limits.max_events)
        {
            return Err(limits::error(
                line,
                "U-LIMIT-EVENTS",
                "event limit exceeded",
            ));
        }
        let (seq, ts, account, provenance) = record.common();
        if self.store.spine.seq.last().is_some_and(|prev| seq <= *prev) {
            return Err(err(line, "seq must be strictly increasing"));
        }
        if self.store.spine.ts.last().is_some_and(|prev| ts < *prev) {
            return Err(err(line, "ts_event must be non-decreasing"));
        }
        if provenance != "observed" {
            return Err(err(
                line,
                "input records must have observed provenance; pass checkpoints separately",
            ));
        }
        if account != self.store.account() {
            self.issue(
                Status::Unsupported,
                Some(seq),
                "multiple-or-unknown-account",
            );
        }
        let (kind, payload, unit) = match record {
            Record::OrderSubmitted {
                client_order_id,
                instrument_id,
                side,
                order_type,
                quantity,
                ..
            } => {
                let order = self.order(&client_order_id, line)?;
                let instrument = self.instrument(&instrument_id, seq);
                let precision = self
                    .store
                    .header
                    .instruments
                    .get(instrument as usize)
                    .map_or(18, |i| i.qty_precision);
                let before = self.store.issues.len();
                let qty = self.number(&quantity, precision, Some(seq), line)?;
                let known = self.store.issues.len() == before;
                if (known && qty.raw() <= 0) || negative_decimal(&quantity) {
                    self.issue(Status::Unsupported, Some(seq), "nonpositive-order-quantity");
                }
                if !matches!(side.as_ref(), "BUY" | "SELL") || order_type != "MARKET" {
                    self.issue(
                        Status::Unsupported,
                        Some(seq),
                        "unsupported-order-type-or-side",
                    );
                }
                // SAFETY-BOUNDS: order() appends all order columns together and returns an in-range ID.
                #[allow(clippy::indexing_slicing)]
                {
                    let o = &mut self.store.orders;
                    if o.instrument[order as usize] == NONE {
                        o.instrument[order as usize] = instrument;
                        o.side[order as usize] = if side == "SELL" {
                            Side::Sell
                        } else {
                            Side::Buy
                        };
                        o.qty_raw[order as usize] = qty.raw();
                        o.qty_scale[order as usize] = qty.scale();
                    }
                }
                (Kind::Submitted, order, self.order_unit(order, line)?)
            }
            Record::OrderAccepted {
                client_order_id,
                venue_order_id,
                ..
            } => {
                let _ = venue_order_id;
                let order = self.order(&client_order_id, line)?;
                (Kind::Accepted, order, self.order_unit(order, line)?)
            }
            Record::OrderRejected {
                client_order_id,
                reason,
                ..
            } => {
                let _ = reason;
                let order = self.order(&client_order_id, line)?;
                (Kind::Rejected, order, self.order_unit(order, line)?)
            }
            Record::OrderCanceled {
                client_order_id, ..
            } => {
                let order = self.order(&client_order_id, line)?;
                (Kind::Canceled, order, self.order_unit(order, line)?)
            }
            Record::OrderFilled {
                client_order_id,
                trade_id,
                last_qty,
                last_px,
                commission,
                commission_currency,
                liquidity_side,
                ..
            } => {
                let order = self.order(&client_order_id, line)?;
                let before = self.store.issues.len();
                let qty = self.number(&last_qty, 18, Some(seq), line)?;
                let known_qty = self.store.issues.len() == before;
                let before = self.store.issues.len();
                let px = self.number(&last_px, 18, Some(seq), line)?;
                let known_px = self.store.issues.len() == before;
                let fee_precision =
                    self.store
                        .header
                        .model
                        .commission_max_scale
                        .unwrap_or_else(|| {
                            self.store
                                .header
                                .currencies
                                .first()
                                .map_or(18, |c| c.precision)
                        });
                let before = self.store.issues.len();
                let fee = self.number(&commission, fee_precision, Some(seq), line)?;
                let known_fee = self.store.issues.len() == before;
                if commission_currency != self.store.header.model.base_currency {
                    self.issue(Status::Unsupported, Some(seq), "commission-currency");
                }
                if (known_qty && qty.raw() <= 0)
                    || (known_px && px.raw() < 0)
                    || (known_fee && fee.raw() < 0)
                    || negative_decimal(&last_qty)
                    || negative_decimal(&last_px)
                    || negative_decimal(&commission)
                {
                    self.issue(
                        Status::Unsupported,
                        Some(seq),
                        "negative-fee-or-nonpositive-fill",
                    );
                }
                if !matches!(liquidity_side.as_ref(), "MAKER" | "TAKER" | "NONE") {
                    self.issue(Status::Unsupported, Some(seq), "liquidity-side");
                }
                let trade = self.intern(&trade_id, line)?;
                let payload = qlrs_schema::checked_index(self.store.fills.order.len())
                    .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
                let f = &mut self.store.fills;
                f.order.push(order);
                f.trade.push(trade);
                f.qty_raw.push(qty.raw());
                f.qty_scale.push(qty.scale());
                f.px_raw.push(px.raw());
                f.px_scale.push(px.scale());
                f.fee_raw.push(fee.raw());
                f.fee_scale.push(fee.scale());
                (Kind::Filled, payload, self.order_unit(order, line)?)
            }
            Record::AccountClaim {
                currency,
                balance_total,
                ..
            } => {
                if currency != self.store.header.model.base_currency {
                    self.issue(Status::Unsupported, Some(seq), "claim-currency");
                }
                let precision = self
                    .store
                    .header
                    .currencies
                    .first()
                    .map_or(18, |c| c.precision);
                let value = self.number(&balance_total, precision, Some(seq), line)?;
                let text = if canonical_decimal(&balance_total, value) {
                    NONE
                } else {
                    self.intern(&balance_total, line)?
                };
                let payload = qlrs_schema::checked_index(self.store.cash_claims.raw.len())
                    .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
                let c = &mut self.store.cash_claims;
                c.raw.push(value.raw());
                c.scale.push(value.scale());
                c.text.push(text);
                let unit = self.new_unit(None, line)?;
                (Kind::CashClaim, payload, unit)
            }
            Record::PositionClaim {
                instrument_id,
                net_qty,
                ..
            } => {
                let instrument = self.instrument(&instrument_id, seq);
                let precision = self
                    .store
                    .header
                    .instruments
                    .get(instrument as usize)
                    .map_or(18, |i| i.qty_precision);
                let value = self.number(&net_qty, precision, Some(seq), line)?;
                let text = if canonical_decimal(&net_qty, value) {
                    NONE
                } else {
                    self.intern(&net_qty, line)?
                };
                let payload = qlrs_schema::checked_index(self.store.position_claims.raw.len())
                    .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
                let c = &mut self.store.position_claims;
                c.instrument.push(instrument);
                c.raw.push(value.raw());
                c.scale.push(value.scale());
                c.text.push(text);
                let unit = self.new_unit(None, line)?;
                (Kind::PositionClaim, payload, unit)
            }
            Record::Unsupported { feature, .. } => {
                self.issue(Status::Unsupported, Some(seq), &feature);
                (Kind::Unsupported, 0, self.new_unit(None, line)?)
            }
        };
        let spine = &mut self.store.spine;
        spine.seq.push(seq);
        spine.ts.push(ts);
        spine.kind.push(kind);
        spine.payload_idx.push(payload);
        spine.unit.push(unit);
        Ok(())
    }
    fn order_unit(&self, order: u32, line: usize) -> Result<u32, InputError> {
        self.store
            .orders
            .unit
            .get(order as usize)
            .copied()
            .ok_or_else(|| err(line, "invalid order reference"))
    }
}
