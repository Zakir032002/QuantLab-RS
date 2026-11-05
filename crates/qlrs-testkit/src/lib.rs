//! Synthetic fixtures and planted mutation operators. Development-only crate.
#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    clippy::module_name_repetitions
)]
use qlrs_num::Fixed;
use qlrs_schema::{Account, CashRounding, Currency, Header, Instrument, Model, Origin, Status};
use std::{fmt, io::Write};

/// Protected planted-defect catalogue identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defect {
    /// Fee charged twice.
    D01,
    /// Fee omitted.
    D02,
    /// One share's notional added.
    D03,
    /// Sell credited as debit.
    D04,
    /// Fill applied twice to cash.
    D05,
    /// Partial fill omitted from position.
    D06,
    /// Fill after cancellation.
    D07,
    /// Overfill.
    D08,
    /// Fill before acceptance.
    D09,
    /// Cash claim scaled by ten.
    D10,
    /// Duplicate trade identifier.
    D11,
    /// Commission in a second currency.
    D12,
    /// Sell exceeding long position.
    D13,
    /// Inexact change under Exact rounding.
    D14,
}
/// All catalogue operators, in their immutable specification order.
pub const DEFECTS: [Defect; 14] = [
    Defect::D01,
    Defect::D02,
    Defect::D03,
    Defect::D04,
    Defect::D05,
    Defect::D06,
    Defect::D07,
    Defect::D08,
    Defect::D09,
    Defect::D10,
    Defect::D11,
    Defect::D12,
    Defect::D13,
    Defect::D14,
];
impl Defect {
    /// Immutable expected classification from the supplied catalogue.
    #[must_use]
    pub const fn expected(self) -> Status {
        match self {
            Self::D12 | Self::D13 => Status::Unsupported,
            Self::D14 => Status::Inconclusive,
            _ => Status::Violation,
        }
    }
    /// Parse a catalogue identifier.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        DEFECTS.into_iter().find(|d| format!("{d:?}") == text)
    }
}
/// Reproducible generated input and its target mutation anchor.
pub struct Fixture {
    /// Strict newline-terminated JSONL bytes.
    pub bytes: Vec<u8>,
    /// Original source sequence where the mutation must be reported.
    pub target_seq: Option<u64>,
}
fn emit(
    out: &mut impl Write,
    seq: &mut u64,
    kind: &str,
    fields: fmt::Arguments<'_>,
) -> Result<u64, String> {
    *seq = seq.checked_add(1).ok_or("sequence overflow")?;
    writeln!(out,"{{\"seq\":{seq},\"ts_event\":{seq},\"type\":\"{kind}\",\"account_id\":\"A\",\"provenance\":\"observed\",{fields}}}").map_err(|e|e.to_string())?;
    Ok(*seq)
}
/// Generate clean traces or apply one catalogue mutation at a chosen lifecycle.
/// Every ordinary lifecycle has submitted, accepted, filled, cash and position.
pub fn generate(
    orders: usize,
    seed: u64,
    mutation: Option<(Defect, usize)>,
) -> Result<Fixture, String> {
    let mut bytes = Vec::new();
    let (target_seq, _) = generate_into(orders, seed, mutation, &mut bytes)?;
    Ok(Fixture { bytes, target_seq })
}
/// Write the existing generator directly to a sink without retaining output.
pub fn generate_into(
    orders: usize,
    seed: u64,
    mutation: Option<(Defect, usize)>,
    mut out: impl Write,
) -> Result<(Option<u64>, u64), String> {
    if orders == 0 {
        return Err("generator requires at least one order".into());
    }
    let target = mutation.map(|(d, index)| {
        let index = index.min(orders.saturating_sub(1));
        (
            d,
            if matches!(d, Defect::D04 | Defect::D11) {
                index.max(1)
            } else {
                index
            },
        )
    });
    if target.is_some_and(|(_, index)| index >= orders) {
        return Err("D04 and D11 require at least two orders".into());
    }
    let header = Header {
        record_type: "header".into(),
        schema: "qlrs-trace/v0".into(),
        model: Model {
            account_type: "CASH".into(),
            base_currency: "INR".into(),
            engine_cash_rounding: CashRounding::Exact,
            commission_max_scale: Some(2),
        },
        currencies: vec![Currency {
            code: "INR".into(),
            precision: 2,
        }],
        instruments: vec![Instrument {
            id: "X".into(),
            quote_currency: "INR".into(),
            qty_precision: 3,
            px_precision: 3,
        }],
        accounts: vec![Account {
            account_id: "A".into(),
            starting_balance: "1000000000000.00".into(),
        }],
        origin: Origin {
            producer: "qlrs-testkit".into(),
            producer_version: env!("CARGO_PKG_VERSION").into(),
            generated_by_qlrs: true,
        },
    };
    out.write_all(&qlrs_io::encode(&header).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut seq = 0_u64;
    let mut target_seq = None;
    let mut cash = 100_000_000_000_000_i128;
    let mut position = 0_i128;
    let price = 100_i128 + i128::from(seed % 11);
    let fee = 1_i128 + i128::from(seed % 7);
    for order in 0..orders {
        let defect = target.filter(|(_, index)| *index == order).map(|(d, _)| d);
        let sell = matches!(defect, Some(Defect::D04 | Defect::D13));
        let qty = if defect == Some(Defect::D13) {
            position + 20_000
        } else if defect == Some(Defect::D06) {
            10_000
        } else if defect == Some(Defect::D14) {
            1
        } else {
            20_000
        };
        let submitted = if defect == Some(Defect::D08) {
            10_000
        } else if defect == Some(Defect::D06) {
            20_000
        } else {
            qty
        };
        let quantity = Fixed::new(submitted, 3)
            .map_err(|e| e.to_string())?
            .to_string();
        let last_qty = Fixed::new(qty, 3).map_err(|e| e.to_string())?.to_string();
        emit(
            &mut out,
            &mut seq,
            "OrderSubmitted",
            format_args!(
                "\"client_order_id\":\"O{order}\",\"instrument_id\":\"X\",\"side\":\"{}\",\"order_type\":\"MARKET\",\"quantity\":\"{quantity}\"",
                if sell { "SELL" } else { "BUY" }
            ),
        )?;
        if defect != Some(Defect::D09) {
            emit(
                &mut out,
                &mut seq,
                "OrderAccepted",
                format_args!("\"client_order_id\":\"O{order}\",\"venue_order_id\":\"V{order}\""),
            )?;
        }
        if defect == Some(Defect::D07) {
            emit(
                &mut out,
                &mut seq,
                "OrderCanceled",
                format_args!("\"client_order_id\":\"O{order}\""),
            )?;
        }
        let trade = if defect == Some(Defect::D11) {
            order.saturating_sub(1)
        } else {
            order
        };
        let last_px = if defect == Some(Defect::D14) {
            "1.005".into()
        } else {
            format!("{price}.00")
        };
        let fill_seq = emit(
            &mut out,
            &mut seq,
            "OrderFilled",
            format_args!(
                "\"client_order_id\":\"O{order}\",\"trade_id\":\"T{trade}\",\"last_qty\":\"{last_qty}\",\"last_px\":\"{last_px}\",\"commission\":\"{fee}.00\",\"commission_currency\":\"{}\",\"liquidity_side\":\"TAKER\"",
                if defect == Some(Defect::D12) {
                    "USD"
                } else {
                    "INR"
                }
            ),
        )?;
        let old_position = position;
        position += if sell { -qty } else { qty };
        let notional = if defect == Some(Defect::D14) {
            0
        } else {
            qty * price / 10
        };
        let delta = if sell { notional } else { -notional } - fee * 100;
        cash += delta;
        let observed = match defect {
            Some(Defect::D01) => cash - fee * 100,
            Some(Defect::D02) => cash + fee * 100,
            Some(Defect::D03) => cash + price * 100,
            Some(Defect::D04) => cash - 2 * notional,
            Some(Defect::D05) => cash + delta,
            Some(Defect::D10) => cash * 10,
            _ => cash,
        };
        let balance = Fixed::new(observed, 2)
            .map_err(|e| e.to_string())?
            .to_string();
        let claim_seq = emit(
            &mut out,
            &mut seq,
            "AccountClaim",
            format_args!("\"currency\":\"INR\",\"balance_total\":\"{balance}\""),
        )?;
        let net = Fixed::new(
            if defect == Some(Defect::D06) {
                old_position
            } else {
                position
            },
            3,
        )
        .map_err(|e| e.to_string())?
        .to_string();
        let pos_seq = emit(
            &mut out,
            &mut seq,
            "PositionClaim",
            format_args!("\"instrument_id\":\"X\",\"net_qty\":\"{net}\""),
        )?;
        if let Some(d) = defect {
            target_seq = Some(match d {
                Defect::D06 => pos_seq,
                Defect::D07
                | Defect::D08
                | Defect::D09
                | Defect::D11
                | Defect::D12
                | Defect::D13
                | Defect::D14 => fill_seq,
                _ => claim_seq,
            });
        }
    }
    Ok((target_seq, seq))
}

mod rich;
pub use rich::{GeneratorConfig, generate_rich, generate_rich_into};

mod shapes;
pub use shapes::{SHAPES, generate_shape, shape_config};

/// Stable seed expansion and generator stream.
pub mod rng;

/// Shared bounded fuzzing and stable corpus-replay oracles.
pub mod oracles;
