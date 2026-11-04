//! Interleaved, sparse-claim, multi-instrument and long-lived order fixtures.
use crate::{Defect, Fixture, emit};
use qlrs_num::Fixed;
use qlrs_schema::{Account, CashRounding, Currency, Header, Instrument, Model, Origin};

/// Deterministic generator dimensions, independent of the defect operator.
// These switches are independent generator axes, not mutually exclusive states.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Ordinary and long-lived orders, excluding inventory bootstrap orders.
    pub orders: usize,
    /// Seed controls prices, fees and scheduling.
    pub seed: u64,
    /// Distinct spot instruments in the same account currency.
    pub instruments: usize,
    /// Simultaneously submitted ordinary orders.
    pub active_orders: usize,
    /// Maximum fills between account/position observation blocks.
    pub claim_every: usize,
    /// Orders partially filled near the start and completed at the end.
    pub long_lived_orders: usize,
    /// Keep a planted cash/position offset in subsequent observations.
    pub carry_errors: bool,
    /// Independent cash and per-instrument observation schedules.
    pub staggered: bool,
    /// Add a second, earlier carried cash error without selecting it as target.
    pub earlier_cash_error: bool,
    /// Perturb every position claim by an increasing instrument-local offset.
    pub dense_violations: bool,
    /// Number of initial buy orders before a scaled sell (zero disables).
    pub fifo_ladder: usize,
    /// A clean parent emits one-lot partials throughout ordinary batches.
    pub parent_partials: bool,
}
impl GeneratorConfig {
    /// Named reproducible profiles used by the extended gate.
    pub fn profile(name: &str, orders: usize, seed: u64) -> Result<Self, String> {
        let (instruments, active_orders, claim_every, long_lived_orders) = match name {
            "interleaved" => (3, 8, 8, 2),
            "long-lived" => (4, 16, 16, 4),
            "sparse" => (4, 16, 32, 2),
            _ => return Err("unknown generator profile".into()),
        };
        Ok(Self {
            orders,
            seed,
            instruments,
            active_orders,
            claim_every,
            long_lived_orders: long_lived_orders.min(orders.saturating_sub(1)),
            carry_errors: true,
            staggered: false,
            earlier_cash_error: false,
            dense_violations: false,
            fifo_ladder: 0,
            parent_partials: false,
        })
    }
}
struct Order {
    id: usize,
    instrument: usize,
    sell: bool,
    defect: Option<Defect>,
    factor: i128,
    parent: bool,
}
struct Generator {
    config: GeneratorConfig,
    out: Vec<u8>,
    seq: u64,
    cash: i128,
    positions: Vec<i128>,
    reserved: Vec<i128>,
    cash_bias: i128,
    position_bias: Vec<i128>,
    fee: i128,
    since_claim: usize,
    pending_cash: bool,
    pending_position: Option<usize>,
    scale_cash: bool,
    target_seq: Option<u64>,
    observations: usize,
}
impl Generator {
    fn price(&self, instrument: usize) -> i128 {
        100 + i128::from(self.config.seed % 11) + i128::try_from(instrument).unwrap_or(0) * 17
    }
    fn money(value: i128) -> Result<String, String> {
        Fixed::new(value, 2)
            .map(|v| v.to_string())
            .map_err(|e| e.to_string())
    }
    fn quantity(value: i128) -> Result<String, String> {
        Fixed::new(value, 3)
            .map(|v| v.to_string())
            .map_err(|e| e.to_string())
    }
    fn flush(&mut self, force: bool) -> Result<(), String> {
        if self.since_claim == 0 && !force {
            return Ok(());
        }
        if self.scale_cash {
            self.cash_bias = (self.cash + self.cash_bias) * 10 - self.cash;
            self.scale_cash = false;
        }
        self.observations = self.observations.saturating_add(1);
        let cash_due =
            !self.config.staggered || force || self.pending_cash || self.observations % 3 == 1;
        let balance = Self::money(self.cash + self.cash_bias)?;
        if cash_due {
            let seq = emit(
                &mut self.out,
                &mut self.seq,
                "AccountClaim",
                format_args!("\"currency\":\"INR\",\"balance_total\":\"{balance}\""),
            )?;
            if self.pending_cash {
                self.target_seq = Some(seq);
                self.pending_cash = false;
            }
        }
        for instrument in 0..self.positions.len() {
            let position_due = if instrument == 0 {
                self.observations % 3 == 2
            } else {
                self.observations % (instrument.saturating_add(3)) == instrument.saturating_sub(1)
            };
            if self.config.staggered && !force && !position_due {
                continue;
            }
            if self.config.dense_violations {
                *self
                    .position_bias
                    .get_mut(instrument)
                    .ok_or("missing position bias")? += 1000;
            }
            let quantity = Self::quantity(
                self.positions
                    .get(instrument)
                    .copied()
                    .ok_or("missing position")?
                    + self
                        .position_bias
                        .get(instrument)
                        .copied()
                        .ok_or("missing position bias")?,
            )?;
            let seq = emit(
                &mut self.out,
                &mut self.seq,
                "PositionClaim",
                format_args!("\"instrument_id\":\"X{instrument}\",\"net_qty\":\"{quantity}\""),
            )?;
            if self.pending_position == Some(instrument) {
                self.target_seq = Some(seq);
                self.pending_position = None;
            }
        }
        if !self.config.carry_errors {
            self.cash_bias = 0;
            self.position_bias.fill(0);
        }
        self.since_claim = 0;
        Ok(())
    }
    fn submit(&mut self, id: usize, mutation: Option<(Defect, usize)>) -> Result<Order, String> {
        let instrument = id % self.config.instruments;
        let defect = mutation
            .filter(|(_, index)| *index == id)
            .map(|(defect, _)| defect);
        let available = self
            .positions
            .get(instrument)
            .copied()
            .ok_or("missing inventory")?
            - self
                .reserved
                .get(instrument)
                .copied()
                .ok_or("missing reservation")?;
        if defect == Some(Defect::D04) && available < 20_000 {
            return Err("D04 target requires unreserved sell inventory".into());
        }
        let ladder_sell = self.config.fifo_ladder > 0 && id == self.config.fifo_ladder;
        let factor = if ladder_sell {
            i128::try_from(self.config.fifo_ladder).map_err(|_| "ladder too long")?
        } else {
            1
        };
        let is_sell = ladder_sell
            || defect == Some(Defect::D04)
            || (id >= self.config.fifo_ladder
                && id / self.config.instruments % 3 == 2
                && available >= 20_000);
        if is_sell {
            let reserved = self
                .reserved
                .get_mut(instrument)
                .ok_or("missing reservation")?;
            *reserved += 20_000 * factor;
        }
        let parent = self.config.parent_partials && id == 0 && defect.is_none();
        let quantity = if parent {
            let parts = self
                .config
                .orders
                .saturating_sub(self.config.long_lived_orders)
                .div_ceil(self.config.active_orders)
                .saturating_add(2);
            Self::quantity(i128::try_from(parts).map_err(|_| "parent too long")? * 1000)?
        } else if defect == Some(Defect::D08) {
            "10.000".to_owned()
        } else {
            Self::quantity(20_000 * factor)?
        };
        emit(
            &mut self.out,
            &mut self.seq,
            "OrderSubmitted",
            format_args!(
                "\"client_order_id\":\"O{id}\",\"instrument_id\":\"X{instrument}\",\"side\":\"{}\",\"order_type\":\"MARKET\",\"quantity\":\"{quantity}\"",
                if is_sell { "SELL" } else { "BUY" }
            ),
        )?;
        if defect != Some(Defect::D09) {
            emit(
                &mut self.out,
                &mut self.seq,
                "OrderAccepted",
                format_args!("\"client_order_id\":\"O{id}\",\"venue_order_id\":\"V{id}\""),
            )?;
        }
        Ok(Order {
            id,
            instrument,
            sell: is_sell,
            defect,
            factor,
            parent,
        })
    }
    fn fill(&mut self, order: &Order, part: usize) -> Result<(), String> {
        let qty = if order.parent {
            1
        } else if part == 0 {
            5_i128
        } else {
            15_i128
        } * order.factor;
        let price = self.price(order.instrument);
        let notional = qty * price * 100;
        let trade_part = if order.defect == Some(Defect::D11) {
            0
        } else {
            part
        };
        let id = order.id;
        let seq = emit(
            &mut self.out,
            &mut self.seq,
            "OrderFilled",
            format_args!(
                "\"client_order_id\":\"O{id}\",\"trade_id\":\"T{id}-{trade_part}\",\"last_qty\":\"{qty}.000\",\"last_px\":\"{price}.000\",\"commission\":\"{}.00\",\"commission_currency\":\"INR\",\"liquidity_side\":\"TAKER\"",
                self.fee
            ),
        )?;
        let position = self
            .positions
            .get_mut(order.instrument)
            .ok_or("missing position")?;
        *position += if order.sell { -qty * 1000 } else { qty * 1000 };
        if order.sell {
            let reserved = self
                .reserved
                .get_mut(order.instrument)
                .ok_or("missing reservation")?;
            *reserved -= qty * 1000;
        }
        let delta = if order.sell { notional } else { -notional } - self.fee * 100;
        self.cash += delta;
        if self.config.earlier_cash_error && order.id == 0 && part == 1 {
            self.cash_bias += 100;
        }
        if part == 0 {
            if order.defect == Some(Defect::D06) {
                *self
                    .position_bias
                    .get_mut(order.instrument)
                    .ok_or("missing bias")? -= qty * 1000;
                self.pending_position = Some(order.instrument);
            }
            if order.defect == Some(Defect::D09) {
                self.target_seq = Some(seq);
            }
        } else {
            match order.defect {
                Some(Defect::D01) => {
                    self.cash_bias -= self.fee * 100;
                    self.pending_cash = true;
                }
                Some(Defect::D02) => {
                    self.cash_bias += self.fee * 100;
                    self.pending_cash = true;
                }
                Some(Defect::D03) => {
                    self.cash_bias += price * 100;
                    self.pending_cash = true;
                }
                Some(Defect::D04) => {
                    self.cash_bias -= 2 * notional;
                    self.pending_cash = true;
                }
                Some(Defect::D05) => {
                    self.cash_bias += delta;
                    self.pending_cash = true;
                }
                Some(Defect::D10) => {
                    self.scale_cash = true;
                    self.pending_cash = true;
                }
                Some(Defect::D07 | Defect::D08 | Defect::D11) => self.target_seq = Some(seq),
                _ => {}
            }
        }
        self.since_claim = self.since_claim.saturating_add(1);
        if self.since_claim >= self.config.claim_every {
            self.flush(false)?;
        }
        if part == 0 && order.defect == Some(Defect::D07) {
            emit(
                &mut self.out,
                &mut self.seq,
                "OrderCanceled",
                format_args!("\"client_order_id\":\"O{id}\""),
            )?;
            self.flush(true)?;
        }
        if part == 0 && order.defect == Some(Defect::D11) {
            self.flush(true)?;
        }
        Ok(())
    }
}
/// Generate all requested adversarial trace dimensions together. D01–D11 keep
/// the financial/model evidence complete; D12–D14 remain in the simple generator.
pub fn generate_rich(
    config: GeneratorConfig,
    mutation: Option<(Defect, usize)>,
) -> Result<Fixture, String> {
    if config.orders == 0
        || !(1..=32).contains(&config.instruments)
        || !(1..=256).contains(&config.active_orders)
        || !(1..=256).contains(&config.claim_every)
        || config.long_lived_orders >= config.orders
    {
        return Err("invalid rich generator dimensions".into());
    }
    if mutation.is_some_and(|(defect, index)| {
        index >= config.orders || matches!(defect, Defect::D12 | Defect::D13 | Defect::D14)
    }) {
        return Err("rich generator requires D01-D11 at an existing order".into());
    }
    let instruments = (0..config.instruments)
        .map(|index| Instrument {
            id: format!("X{index}"),
            quote_currency: "INR".into(),
            qty_precision: 3,
            px_precision: 3,
        })
        .collect();
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
        instruments,
        accounts: vec![Account {
            account_id: "A".into(),
            starting_balance: "1000000000000.00".into(),
        }],
        origin: Origin {
            producer: "qlrs-testkit-rich".into(),
            producer_version: env!("CARGO_PKG_VERSION").into(),
            generated_by_qlrs: true,
        },
    };
    let count = config.instruments;
    let fee = 1 + i128::from(config.seed % 7);
    let mut generator = Generator {
        config,
        out: qlrs_io::encode(&header).map_err(|e| e.to_string())?,
        seq: 0,
        cash: 100_000_000_000_000,
        positions: vec![0; count],
        reserved: vec![0; count],
        cash_bias: 0,
        position_bias: vec![0; count],
        fee,
        since_claim: 0,
        pending_cash: false,
        pending_position: None,
        scale_cash: false,
        target_seq: None,
        observations: 0,
    };
    generator.flush(true)?;
    // Observed bootstrap orders provide long-only sell inventory. Checkpoints
    // carry that inventory so witnesses need not retain bootstrap buy history.
    for instrument in 0..count {
        let price = generator.price(instrument);
        emit(
            &mut generator.out,
            &mut generator.seq,
            "OrderSubmitted",
            format_args!(
                "\"client_order_id\":\"BOOT{instrument}\",\"instrument_id\":\"X{instrument}\",\"side\":\"BUY\",\"order_type\":\"MARKET\",\"quantity\":\"1000.000\""
            ),
        )?;
        emit(
            &mut generator.out,
            &mut generator.seq,
            "OrderAccepted",
            format_args!(
                "\"client_order_id\":\"BOOT{instrument}\",\"venue_order_id\":\"B{instrument}\""
            ),
        )?;
        emit(
            &mut generator.out,
            &mut generator.seq,
            "OrderFilled",
            format_args!(
                "\"client_order_id\":\"BOOT{instrument}\",\"trade_id\":\"BOOT-T{instrument}\",\"last_qty\":\"1000.000\",\"last_px\":\"{price}.000\",\"commission\":\"0.00\",\"commission_currency\":\"INR\",\"liquidity_side\":\"TAKER\""
            ),
        )?;
        generator.cash -= 1000 * price * 100;
        *generator
            .positions
            .get_mut(instrument)
            .ok_or("missing position")? += 1_000_000;
        generator.since_claim = generator.since_claim.saturating_add(1);
    }
    generator.flush(false)?;
    let mut long_lived = Vec::new();
    for id in 0..generator.config.long_lived_orders {
        let order = generator.submit(id, mutation)?;
        generator.fill(&order, 0)?;
        long_lived.push(order);
    }
    let mut next = generator.config.long_lived_orders;
    while next < generator.config.orders {
        let stop = next
            .saturating_add(generator.config.active_orders)
            .min(generator.config.orders);
        let mut active = Vec::new();
        for id in next..stop {
            active.push(generator.submit(id, mutation)?);
        }
        for order in active.iter().rev() {
            generator.fill(order, 0)?;
        }
        let rotation = usize::try_from(
            generator.config.seed % u64::try_from(active.len()).map_err(|_| "batch too large")?,
        )
        .map_err(|_| "invalid rotation")?;
        for offset in 0..active.len() {
            let index = offset.saturating_add(rotation) % active.len();
            generator.fill(active.get(index).ok_or("invalid order")?, 1)?;
        }
        if generator.config.parent_partials {
            for order in &long_lived {
                generator.fill(order, next.saturating_add(2))?;
            }
        }
        next = stop;
    }
    for order in long_lived.iter().rev() {
        generator.fill(order, 1)?;
    }
    generator.flush(true)?;
    if mutation.is_some() && generator.target_seq.is_none() {
        return Err("defect target was not emitted".into());
    }
    Ok(Fixture {
        bytes: generator.out,
        target_seq: generator.target_seq,
    })
}
