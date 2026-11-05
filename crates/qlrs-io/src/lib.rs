//! Strict JSONL boundary, borrowed parallel parsing and verbatim source writing.
#![forbid(unsafe_code)]
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]
mod limits;
pub use limits::{Limits, SCHEMA_KEYS, validate_line};
use qlrs_num::{Fixed, NumericError};
use qlrs_schema::{Account, AliveMask, Header, Issue, Kind, NONE, Side, Status, TraceStore, Unit};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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

/// Original bytes are isolated at the I/O boundary, not retained in replay columns.
#[derive(Debug)]
pub struct ParsedTrace {
    /// Compact replay/check/reduction store.
    pub store: TraceStore,
    /// Original JSONL, including all whitespace.
    pub source: Vec<u8>,
    /// One range for the header, followed by one per event; includes newline.
    pub lines: Vec<std::ops::Range<usize>>,
}
impl ParsedTrace {
    /// Read a trace using safe buffered file I/O.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, InputError> {
        Self::parse(fs::read(path).map_err(|e| err(0, e.to_string()))?)
    }
    /// Strictly parse a complete UTF-8 JSONL buffer.
    pub fn parse(source: Vec<u8>) -> Result<Self, InputError> {
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
                lines.push(start..end + 1);
                start = end + 1;
            }
        }
        let first = lines.first().ok_or_else(|| err(1, "missing header"))?;
        let header: Header = serde_json::from_slice(
            source
                .get(first.clone())
                .ok_or_else(|| err(1, "missing header"))?,
        )
        .map_err(|e| err(1, e.to_string()))?;
        if header.record_type != "header" || header.schema != "qlrs-trace/v0" {
            return Err(err(1, "expected header schema qlrs-trace/v0"));
        }
        if header.accounts.is_empty() || header.currencies.is_empty() {
            return Err(err(1, "accounts and currencies cannot be empty"));
        }
        let mut builder = Builder::new(header)?;
        // Bounded parallel batches borrow the immutable source; merge in source order.
        for (batch_index, batch) in lines.get(1..).unwrap_or_default().chunks(8192).enumerate() {
            let parse = |(offset, range): (usize, &std::ops::Range<usize>)| {
                let line = batch_index * 8192 + offset + 2;
                let bytes = source
                    .get(range.clone())
                    .ok_or_else(|| err(line, "invalid source range"))?;
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
        })
    }
    /// Parse with explicitly selected resource limits. Existing parse defaults
    /// remain unchanged pending the recorded R9 review.
    pub fn parse_with_limits(source: Vec<u8>, limits: Limits) -> Result<Self, InputError> {
        Self::from_limited_reader(std::io::Cursor::new(source), limits)
    }
    /// Read a regular file with line bounds enforced before whole-line buffering.
    pub fn read_with_limits(path: impl AsRef<Path>, limits: Limits) -> Result<Self, InputError> {
        limits.validate()?;
        let metadata = fs::metadata(path.as_ref()).map_err(|e| err(0, e.to_string()))?;
        if !metadata.is_file() {
            return Err(limits::error(
                0,
                "E-INPUT-NOT-SEEKABLE",
                "input is not a regular file",
            ));
        }
        if metadata.len() > limits.max_input_bytes {
            return Err(limits::error(
                0,
                "E-LIMIT-INPUT-BYTES",
                "input exceeds byte limit",
            ));
        }
        let file = fs::File::open(path).map_err(|e| err(0, e.to_string()))?;
        Self::from_limited_reader(std::io::BufReader::with_capacity(64 << 10, file), limits)
    }
    fn from_limited_reader(
        mut reader: impl std::io::BufRead,
        limits: Limits,
    ) -> Result<Self, InputError> {
        use std::io::{BufRead, Read};
        limits.validate()?;
        let mut source = Vec::new();
        let mut lines = Vec::new();
        let mut buffer = Vec::new();
        let mut builder: Option<Builder> = None;
        loop {
            buffer.clear();
            let cap = u64::try_from(limits.max_line_bytes)
                .map_err(|e| err(0, e.to_string()))?
                .saturating_add(1);
            let count = reader
                .by_ref()
                .take(cap)
                .read_until(b'\n', &mut buffer)
                .map_err(|e| err(0, e.to_string()))?;
            if count == 0 {
                break;
            }
            let line = lines.len().saturating_add(1);
            validate_line(&buffer, line, limits)?;
            if buffer.last() != Some(&b'\n') {
                return Err(limits::error(
                    line,
                    "E-NEWLINE",
                    "trace must end with a newline",
                ));
            }
            let end = source
                .len()
                .checked_add(buffer.len())
                .ok_or_else(|| limits::error(line, "U-LIMIT-INDEX", "source length overflow"))?;
            if u64::try_from(end).map_err(|e| err(line, e.to_string()))? > limits.max_input_bytes {
                return Err(limits::error(
                    line,
                    "E-LIMIT-INPUT-BYTES",
                    "input exceeds byte limit",
                ));
            }
            if limits.memory_budget_bytes.is_some_and(|budget| {
                u64::try_from(end)
                    .ok()
                    .and_then(|bytes| bytes.checked_mul(2))
                    .and_then(|bytes| {
                        bytes.checked_add(u64::try_from(line).ok()?.checked_mul(160)?)
                    })
                    .and_then(|bytes| bytes.checked_add(64 << 20))
                    .is_none_or(|estimate| estimate > budget)
            }) {
                return Err(limits::error(
                    line,
                    "I-BUDGET-MEMORY",
                    "estimated memory budget exceeded",
                ));
            }
            if let Some(builder) = builder.as_mut() {
                builder.push(parse_record(&buffer, line)?, line)?;
            } else {
                if buffer.starts_with(&[0xef, 0xbb, 0xbf]) {
                    return Err(err(1, "UTF-8 BOM is forbidden"));
                }
                let header: Header = decode(&buffer).map_err(|mut error| {
                    error.line = line;
                    error
                })?;
                if header.record_type != "header" || header.schema != "qlrs-trace/v0" {
                    return Err(err(1, "expected header schema qlrs-trace/v0"));
                }
                if header.accounts.is_empty() || header.currencies.is_empty() {
                    return Err(err(1, "accounts and currencies cannot be empty"));
                }
                let mut value = Builder::new(header)?;
                value.limits = Some(limits);
                builder = Some(value);
            }
            source
                .try_reserve(buffer.len())
                .map_err(|e| limits::error(line, "I-BUDGET-MEMORY", &e.to_string()))?;
            lines
                .try_reserve(1)
                .map_err(|e| limits::error(line, "I-BUDGET-MEMORY", &e.to_string()))?;
            lines.push(source.len()..end);
            source.extend_from_slice(&buffer);
        }
        let mut builder = builder.ok_or_else(|| err(1, "missing header"))?;
        builder.store.compact();
        Ok(Self {
            store: builder.store,
            source,
            lines,
        })
    }
    /// Write the original pinned header and kept observed lines byte-for-byte.
    pub fn write_kept(&self, alive: &AliveMask, mut writer: impl Write) -> Result<(), InputError> {
        for (line, range) in self.lines.iter().enumerate() {
            let keep = line == 0
                || self
                    .store
                    .spine
                    .seq
                    .get(line - 1)
                    .is_some_and(|_| alive.contains_event(&self.store, line - 1));
            if keep {
                writer
                    .write_all(
                        self.source
                            .get(range.clone())
                            .ok_or_else(|| err(line, "source range missing"))?,
                    )
                    .map_err(|e| err(0, e.to_string()))?;
            }
        }
        Ok(())
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
struct Builder {
    store: TraceStore,
    orders: HashMap<u32, u32>,
    instruments: HashMap<String, u32>,
    limits: Option<Limits>,
}
impl Builder {
    fn new(header: Header) -> Result<Self, InputError> {
        let mut instruments = HashMap::new();
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
            orders: HashMap::new(),
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
        self.store.units.push(Unit {
            order,
            first_event: self.store.len(),
        });
        Ok(id)
    }
    fn order(&mut self, text: &str, line: usize) -> Result<u32, InputError> {
        let symbol = self.intern(text, line)?;
        if let Some(id) = self.orders.get(&symbol) {
            return Ok(*id);
        }
        let id = qlrs_schema::checked_index(self.store.orders.id.len())
            .map_err(|_| limits::error(line, "U-LIMIT-INDEX", "column index exhausted"))?;
        let unit = self.new_unit(Some(id), line)?;
        self.orders.insert(symbol, id);
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
