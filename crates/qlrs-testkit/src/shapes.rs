//! Seeded Part 2 shape API; scheduling dimensions are independent of the defect.
use crate::{Defect, Fixture, GeneratorConfig, generate, generate_rich};
/// Named shapes used verbatim in the gate output.
pub const SHAPES: [&str; 9] = ["S1", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9"];
/// Generate approximately the requested event budget. Actual event counts are
/// reported by the gate; header/bootstrap/final boundaries add fixed overhead.
pub fn generate_shape(
    shape: &str,
    seed: u64,
    n_events: usize,
    defect: Option<Defect>,
) -> Result<Fixture, String> {
    let orders = n_events.div_ceil(if shape == "S1" { 5 } else { 4 }).max(16);
    if shape == "S1" {
        return add_removable_context(generate(
            orders,
            seed,
            defect.map(|defect| (defect, orders / 2)),
        )?);
    }
    let config = shape_config(shape, seed, orders)?;
    let index = if shape == "S8" && defect == Some(Defect::D04) {
        config.fifo_ladder
    } else {
        orders / 2
    };
    let mut fixture = generate_rich(config, defect.map(|defect| (defect, index)))?;
    if shape == "S9" {
        let lines: Vec<_> = fixture
            .bytes
            .split_inclusive(|byte| *byte == b'\n')
            .collect();
        let last_cash = lines.iter().rposition(|line| {
            std::str::from_utf8(line).is_ok_and(|text| text.contains("\"type\":\"AccountClaim\""))
        });
        fixture.bytes = lines
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != last_cash)
            .flat_map(|(_, line)| line.iter().copied())
            .collect();
    }
    add_removable_context(fixture)
}

/// Scheduling configuration shared by small fixtures and streaming presets.
pub fn shape_config(shape: &str, seed: u64, orders: usize) -> Result<GeneratorConfig, String> {
    let mut config = GeneratorConfig::profile("interleaved", orders, seed)?;
    config.instruments = 1;
    config.long_lived_orders = 0;
    config.active_orders = 2 + usize::try_from(seed % 15).map_err(|_| "invalid seed")?;
    match shape {
        "S2" => {}
        "S3" => {
            config.long_lived_orders = 1;
            config.parent_partials = true;
            config.claim_every = 5;
        }
        "S4" => {
            config.claim_every = 5 + usize::try_from(seed % 46).map_err(|_| "invalid seed")?;
        }
        "S5" => {
            config.instruments = 2 + usize::try_from(seed % 7).map_err(|_| "invalid seed")?;
        }
        "S6" => {
            config.earlier_cash_error = true;
        }
        "S7" => {
            config.dense_violations = true;
            config.claim_every = 1;
        }
        "S8" => {
            config.fifo_ladder =
                (10 + usize::try_from(seed % 491).map_err(|_| "invalid seed")?).min(orders / 2);
        }
        "S9" => {
            config.instruments = 3;
            config.staggered = true;
            config.claim_every = 3;
        }
        _ => return Err("unknown Part 2 shape".into()),
    }
    Ok(config)
}

fn add_removable_context(fixture: Fixture) -> Result<Fixture, String> {
    let Some(target) = fixture.target_seq else {
        return Ok(fixture);
    };
    let trace = qlrs_io::ParsedTrace::parse(fixture.bytes).map_err(|e| e.to_string())?;
    let event = trace
        .store
        .spine
        .seq
        .binary_search(&target)
        .map_err(|_| "missing generated target")?;
    let instrument =
        if trace.store.spine.kind.get(event) == Some(&qlrs_schema::Kind::PositionClaim) {
            trace
                .store
                .spine
                .payload_idx
                .get(event)
                .and_then(|payload| {
                    trace
                        .store
                        .position_claims
                        .instrument
                        .get(*payload as usize)
                })
                .and_then(|instrument| trace.store.header.instruments.get(*instrument as usize))
        } else {
            trace.store.header.instruments.first()
        }
        .ok_or("no instrument")?;
    let mut out = Vec::with_capacity(trace.source.len().saturating_add(512));
    let header = trace
        .lines
        .first()
        .and_then(|range| trace.source.get(range.clone()))
        .ok_or("missing header")?;
    out.extend_from_slice(header);
    let mut next = 0_u64;
    let mut target_seq = None;
    for (event, old) in trace.store.spine.seq.iter().enumerate() {
        if *old == target {
            crate::emit(
                &mut out,
                &mut next,
                "OrderSubmitted",
                format_args!(
                    "\"client_order_id\":\"NOISE\",\"instrument_id\":\"{}\",\"side\":\"BUY\",\"order_type\":\"MARKET\",\"quantity\":\"1.000\"",
                    instrument.id
                ),
            )?;
            crate::emit(
                &mut out,
                &mut next,
                "OrderAccepted",
                format_args!("\"client_order_id\":\"NOISE\",\"venue_order_id\":\"NOISE-V\""),
            )?;
        }
        next = next.checked_add(1).ok_or("sequence overflow")?;
        let line = trace
            .lines
            .get(event.saturating_add(1))
            .and_then(|range| trace.source.get(range.clone()))
            .ok_or("missing event")?;
        let text = std::str::from_utf8(line).map_err(|e| e.to_string())?;
        let text = text
            .replacen(&format!("\"seq\":{old},"), &format!("\"seq\":{next},"), 1)
            .replacen(
                &format!("\"ts_event\":{old},"),
                &format!("\"ts_event\":{next},"),
                1,
            );
        out.extend_from_slice(text.as_bytes());
        if *old == target {
            target_seq = Some(next);
        }
    }
    Ok(Fixture {
        bytes: out,
        target_seq,
    })
}
