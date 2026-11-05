//! Bounded preflight with fixed masks for known keys; serde remains the parser.
use crate::{ErrorKind, InputError, err};
use std::{borrow::Cow, collections::BTreeSet};

/// Trace and bundle limits. Defaults are the Part 3 proposal; callers opt in
/// until the recorded R9 default-verdict review is approved.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum line length including its newline.
    pub max_line_bytes: usize,
    /// Maximum simultaneously open JSON containers.
    pub max_json_depth: usize,
    /// Maximum numeric token or decimal-string length.
    pub max_numeric_literal_len: usize,
    /// Maximum decimal fractional digits.
    pub max_decimal_scale: usize,
    /// Maximum decoded string length in bytes.
    pub max_string_bytes: usize,
    /// Maximum distinct interned identifiers.
    pub max_interned: usize,
    /// Maximum records excluding the header.
    pub max_events: usize,
    /// Maximum input length.
    pub max_input_bytes: u64,
    /// Optional estimated allocation budget before store construction.
    pub memory_budget_bytes: Option<u64>,
    /// Maximum bundle directory entries.
    pub max_entries: usize,
    /// Maximum individual bundle file size.
    pub max_entry_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_line_bytes: 1 << 20,
            max_json_depth: 4,
            max_numeric_literal_len: 40,
            max_decimal_scale: 18,
            max_string_bytes: 256,
            max_interned: 1 << 24,
            max_events: 200_000_000,
            max_input_bytes: 64 << 30,
            memory_budget_bytes: None,
            max_entries: 10_000,
            max_entry_bytes: 1 << 30,
        }
    }
}
impl Limits {
    /// Reject configurations beyond the documented hard ceilings.
    pub fn validate(self) -> Result<(), InputError> {
        if self.max_line_bytes > 16 << 20
            || self.max_json_depth > 8
            || self.max_numeric_literal_len > 64
            || self.max_decimal_scale > 18
            || self.max_string_bytes > 4096
            || self.max_interned > 4_294_967_294
            || self.max_events > 4_294_967_294
        {
            return Err(error(0, "E-LIMIT-CONFIG", "limit exceeds its hard ceiling"));
        }
        Ok(())
    }
    /// Check resource bounds on a decoded decimal; grammar remains Fixed's job.
    pub fn number(self, value: &str, line: usize) -> Result<(), InputError> {
        if value.len() > self.max_numeric_literal_len {
            return Err(error(
                line,
                "E-LIMIT-NUMBER-LEN",
                "numeric literal too long",
            ));
        }
        if value
            .split_once('.')
            .is_some_and(|(_, tail)| tail.len() > self.max_decimal_scale)
        {
            return Err(error(line, "E-SCALE", "decimal scale exceeds limit"));
        }
        Ok(())
    }
    pub(crate) fn string(self, value: &str, line: usize) -> Result<(), InputError> {
        if value.len() > self.max_string_bytes {
            return Err(error(line, "E-LIMIT-STRING", "decoded string too long"));
        }
        Ok(())
    }
}
pub(crate) fn error(line: usize, code: &'static str, message: &str) -> InputError {
    let mut error = err(line, message);
    error.code = code;
    error.kind = match code {
        "U-LIMIT-EVENTS" | "U-LIMIT-INTERNED" | "U-LIMIT-INDEX" => ErrorKind::Unsupported,
        "I-BUDGET-MEMORY" => ErrorKind::Budget,
        _ => ErrorKind::Input,
    };
    error
}

// One canonical schema-key list also drives the fuzz dictionary. A bitset tracks
// common keys; only unknown extension keys need storage for duplicate detection.
/// Header/event keys, in stable dictionary/duplicate-mask order.
pub const SCHEMA_KEYS: &[&str] = &[
    "type",
    "seq",
    "ts_event",
    "account_id",
    "provenance",
    "client_order_id",
    "instrument_id",
    "side",
    "order_type",
    "quantity",
    "venue_order_id",
    "reason",
    "trade_id",
    "last_qty",
    "last_px",
    "commission",
    "commission_currency",
    "liquidity_side",
    "balance_total",
    "currency",
    "net_qty",
    "feature",
    "schema",
    "model",
    "currencies",
    "instruments",
    "accounts",
    "origin",
    "account_type",
    "base_currency",
    "engine_cash_rounding",
    "commission_max_scale",
    "code",
    "precision",
    "id",
    "quote_currency",
    "qty_precision",
    "px_precision",
    "starting_balance",
    "producer",
    "producer_version",
    "generated_by_qlrs",
];
#[derive(Default)]
struct Frame<'a> {
    object: bool,
    key: bool,
    numeric: bool,
    seen: u64,
    extensions: BTreeSet<Cow<'a, str>>,
}

/// Check resource bounds and decoded duplicate keys without building a JSON
/// value tree. The existing serde decoder supplies full grammar/schema checks.
pub fn validate_line(bytes: &[u8], line: usize, limits: Limits) -> Result<(), InputError> {
    if bytes.len() > limits.max_line_bytes {
        return Err(error(line, "E-LIMIT-LINE-BYTES", "line exceeds byte limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| error(line, "E-UTF8", &e.to_string()))?;
    let mut frames: [Frame<'_>; 8] = std::array::from_fn(|_| Frame::default());
    let mut depth = 0_usize;
    let mut cursor = 0_usize;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'{' | b'[' => {
                if depth >= limits.max_json_depth {
                    return Err(error(line, "E-LIMIT-DEPTH", "JSON nesting exceeds limit"));
                }
                let frame = frames.get_mut(depth).ok_or_else(|| {
                    error(line, "E-LIMIT-DEPTH", "JSON nesting exceeds hard limit")
                })?;
                *frame = Frame {
                    object: byte == b'{',
                    key: byte == b'{',
                    ..Frame::default()
                };
                depth = depth.saturating_add(1);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            b',' => {
                if let Some(frame) = depth.checked_sub(1).and_then(|i| frames.get_mut(i)) {
                    frame.key = frame.object;
                    frame.numeric = false;
                }
            }
            b'"' => {
                let start = cursor;
                cursor = cursor.saturating_add(1);
                let mut escaped = false;
                loop {
                    match bytes.get(cursor) {
                        Some(b'"') => break,
                        Some(b'\\') => {
                            escaped = true;
                            cursor = cursor.saturating_add(1);
                        }
                        None => return Err(error(line, "E-JSON", "unterminated JSON string")),
                        _ => {}
                    }
                    cursor = cursor.saturating_add(1);
                }
                let end = cursor.saturating_add(1);
                let value: Cow<'_, str> = if escaped {
                    serde_json::from_slice(
                        bytes
                            .get(start..end)
                            .ok_or_else(|| err(line, "invalid string range"))?,
                    )
                    .map_err(|e| {
                        error(
                            line,
                            if e.to_string().contains("surrogate")
                                || e.to_string().contains("unicode")
                                || e.to_string().contains("hex escape")
                            {
                                "E-UTF8"
                            } else {
                                "E-JSON"
                            },
                            &e.to_string(),
                        )
                    })?
                } else {
                    Cow::Borrowed(
                        text.get(start.saturating_add(1)..cursor)
                            .ok_or_else(|| error(line, "E-JSON", "invalid string range"))?,
                    )
                };
                if depth
                    .checked_sub(1)
                    .and_then(|i| frames.get(i))
                    .is_some_and(|frame| frame.object && !frame.key && frame.numeric)
                {
                    limits.number(&value, line)?;
                }
                limits.string(&value, line)?;
                if let Some(frame) = depth.checked_sub(1).and_then(|i| frames.get_mut(i))
                    && frame.object
                    && frame.key
                {
                    if let Some(index) = SCHEMA_KEYS.iter().position(|key| *key == value) {
                        let bit = 1_u64
                            .checked_shl(
                                u32::try_from(index)
                                    .map_err(|_| err(line, "schema key overflow"))?,
                            )
                            .ok_or_else(|| err(line, "schema key overflow"))?;
                        if frame.seen & bit != 0 {
                            return Err(error(line, "E-DUPLICATE-KEY", "duplicate decoded key"));
                        }
                        frame.seen |= bit;
                    } else if !frame.extensions.insert(value.clone()) {
                        return Err(error(line, "E-DUPLICATE-KEY", "duplicate decoded key"));
                    }
                    frame.key = false;
                    frame.numeric = matches!(
                        value.as_ref(),
                        "quantity"
                            | "last_qty"
                            | "last_px"
                            | "commission"
                            | "balance_total"
                            | "net_qty"
                            | "starting_balance"
                    );
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = cursor;
                while bytes.get(cursor).is_some_and(|byte| {
                    byte.is_ascii_digit() || matches!(*byte, b'-' | b'+' | b'.' | b'e' | b'E')
                }) {
                    cursor = cursor.saturating_add(1);
                }
                if cursor.saturating_sub(start) > limits.max_numeric_literal_len {
                    return Err(error(line, "E-LIMIT-NUMBER-LEN", "numeric token too long"));
                }
                continue;
            }
            _ => {}
        }
        cursor = cursor.saturating_add(1);
    }
    Ok(())
}
