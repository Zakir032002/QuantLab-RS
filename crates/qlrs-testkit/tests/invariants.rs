//! Gate A-synthetic: fixed catalogue expectations and clean-trace properties.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use proptest::prelude::*;
use qlrs_check::{Checker, Invariant};
use qlrs_ledger::Ledger;
use qlrs_schema::{AliveMask, Status};
use qlrs_testkit::{DEFECTS, Defect, generate};
fn report(bytes: Vec<u8>) -> qlrs_check::Report {
    let mut trace = qlrs_io::ParsedTrace::parse(bytes).unwrap();
    Ledger::prepare(&mut trace.store);
    Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        None,
        false,
    )
}
#[test]
fn req_check_001_catalogue_statuses_and_invariants() {
    for defect in DEFECTS {
        let input = generate(5, 0, Some((defect, 2))).unwrap();
        let anchor = input.target_seq.unwrap();
        let result = report(input.bytes);
        assert_eq!(result.status, defect.expected(), "{defect:?}: {result:#?}");
        if defect.expected() == Status::Violation {
            let finding = result
                .at_seq(anchor)
                .unwrap_or_else(|| panic!("missing anchor for {defect:?}: {result:#?}"));
            let invariant = if defect == Defect::D06 {
                Invariant::I2
            } else if matches!(
                defect,
                Defect::D07 | Defect::D08 | Defect::D09 | Defect::D11
            ) {
                Invariant::I3
            } else {
                Invariant::I1
            };
            assert_eq!(
                finding.instance_key.as_ref().unwrap().invariant_id,
                invariant
            );
        } else {
            assert!(
                !result
                    .findings
                    .iter()
                    .any(|f| f.status == Status::Violation)
            );
        }
    }
}
proptest! {
 #![proptest_config(ProptestConfig { cases:1000, ..ProptestConfig::default() })]
 #[test]
 fn req_check_002_clean_generated_consistent(seed in any::<u64>(),count in 2usize..24){
   let result=report(generate(count,seed,None).unwrap().bytes);prop_assert_eq!(result.status,Status::Consistent,"{:?}",result);
 }
 #[test]
 fn req_check_003_all_defects_1000_injection_points(seed in any::<u64>(),index in 1usize..16){
   for defect in DEFECTS {let fixture=generate(18,seed,Some((defect,index))).unwrap();let result=report(fixture.bytes);prop_assert_eq!(result.status,defect.expected(),"{:?}: {:?}",defect,result);}
 }
}
#[test]
fn req_io_001_strict_unknown_fields_decimal_types_and_sequences() {
    let bytes = generate(2, 0, None).unwrap().bytes;
    let text = String::from_utf8(bytes).unwrap();
    for broken in [
        text.replacen("\"schema\":", "\"surprise\":true,\"schema\":", 1),
        text.replacen("\"quantity\":\"20.000\"", "\"quantity\":20", 1),
        text.replacen("\"seq\":2", "\"seq\":1", 1),
        text.trim_end().into(),
    ] {
        assert!(qlrs_io::ParsedTrace::parse(broken.into_bytes()).is_err());
    }
}
#[test]
fn req_io_002_roundtrip_verbatim() {
    let input = generate(25, 0, None).unwrap().bytes;
    let trace = qlrs_io::ParsedTrace::parse(input.clone()).unwrap();
    let mut written = Vec::new();
    trace
        .write_kept(&AliveMask::all(trace.store.units.len()), &mut written)
        .unwrap();
    assert_eq!(written, input);
}
#[test]
fn req_check_004_residual_relations_are_hints() {
    for (defect, relation) in [
        (Defect::D01, "-fee"),
        (Defect::D02, "+fee"),
        (Defect::D03, "+1*last_px"),
    ] {
        let input = generate(3, 0, Some((defect, 1))).unwrap();
        let anchor = input.target_seq.unwrap();
        let result = report(input.bytes);
        assert_eq!(result.at_seq(anchor).unwrap().relation, relation);
    }
    let input = generate(3, 0, Some((Defect::D10, 1))).unwrap();
    let anchor = input.target_seq.unwrap();
    assert!(
        report(input.bytes)
            .at_seq(anchor)
            .unwrap()
            .relation
            .starts_with("scale-error:")
    );
}
#[test]
fn req_check_005_clean_corpus_and_flat_position() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/clean");
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        let result = report(std::fs::read(&path).unwrap());
        assert_eq!(result.status, Status::Consistent, "{path:?}: {result:#?}");
        if path.file_name().unwrap() == "C01-worked-example.jsonl" {
            assert_eq!(result.expected_cash, "7990.00");
        }
    }
}
#[test]
fn req_check_006_unknown_event_is_unsupported_not_input_error() {
    let mut text = String::from_utf8(generate(1, 0, None).unwrap().bytes).unwrap();
    text.push_str("{\"seq\":6,\"ts_event\":6,\"type\":\"CorporateAction\",\"account_id\":\"A\",\"provenance\":\"observed\",\"action\":\"split\"}\n");
    let result = report(text.into_bytes());
    assert_eq!(result.status, Status::Unsupported);
    assert!(
        !result
            .findings
            .iter()
            .any(|f| f.status == Status::Violation)
    );
}
#[test]
fn req_check_007_partial_and_status_precedence() {
    let mut text =
        String::from_utf8(generate(2, 0, Some((Defect::D03, 0))).unwrap().bytes).unwrap();
    text.push_str("{\"seq\":11,\"ts_event\":11,\"type\":\"Funding\",\"account_id\":\"A\",\"provenance\":\"observed\"}\n");
    let mut input = qlrs_io::ParsedTrace::parse(text.into_bytes()).unwrap();
    Ledger::prepare(&mut input.store);
    let mask = AliveMask::all(input.store.units.len());
    let full = Checker::check(&input.store, &mask, None, false);
    assert_eq!(full.status, Status::Unsupported);
    assert_eq!(full.keys().count(), 0);
    let partial = Checker::check(&input.store, &mask, None, true);
    assert_eq!(partial.status, Status::Unsupported);
    assert!(partial.keys().next().is_some());
}
#[test]
fn req_io_003_verbatim_noncanonical_decimal_evidence() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/clean/C01-worked-example.jsonl");
    let text = std::fs::read_to_string(path)
        .unwrap()
        .replace("7990.00", "08090.00");
    let result = report(text.into_bytes());
    assert_eq!(result.status, Status::Violation);
    assert_eq!(
        result.findings.first().unwrap().observed.as_deref(),
        Some("08090.00")
    );
}
#[test]
fn req_io_004_malformed_overflow_still_input_error() {
    let text = String::from_utf8(generate(1, 0, None).unwrap().bytes)
        .unwrap()
        .replace(
            "\"commission\":\"1.00\"",
            "\"commission\":\"99999999999999999999999999999999999999999999e2\"",
        );
    let error = qlrs_io::ParsedTrace::parse(text.into_bytes()).unwrap_err();
    assert_eq!(error.line, 4);
}
#[test]
fn req_check_008_lifecycle_subcodes_and_supported_terminal_states() {
    for (defect, code) in [
        (Defect::D07, "event-after-terminal"),
        (Defect::D08, "overfill"),
        (Defect::D09, "fill-before-accept"),
        (Defect::D11, "duplicate-trade-id"),
    ] {
        let input = generate(4, 0, Some((defect, 2))).unwrap();
        let anchor = input.target_seq.unwrap();
        let result = report(input.bytes);
        assert_eq!(
            result
                .at_seq(anchor)
                .unwrap()
                .instance_key
                .as_ref()
                .unwrap()
                .sub_code,
            code
        );
    }
    let fixture = generate(1, 0, None).unwrap();
    let text = String::from_utf8(fixture.bytes).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    let orphan = format!(
        "{header}\n{{\"seq\":0,\"ts_event\":0,\"type\":\"OrderAccepted\",\"account_id\":\"A\",\"provenance\":\"observed\",\"client_order_id\":\"missing\",\"venue_order_id\":\"v\"}}\n"
    );
    let result = report(orphan.into_bytes());
    assert_eq!(result.status, Status::Violation);
    assert_eq!(
        result
            .at_seq(0)
            .unwrap()
            .instance_key
            .as_ref()
            .unwrap()
            .sub_code,
        "unknown-order"
    );
    let submitted = lines.next().unwrap();
    let rejected = format!(
        "{header}\n{submitted}\n{{\"seq\":2,\"ts_event\":2,\"type\":\"OrderRejected\",\"account_id\":\"A\",\"provenance\":\"observed\",\"client_order_id\":\"O0\",\"reason\":\"no liquidity\"}}\n"
    );
    assert_eq!(report(rejected.into_bytes()).status, Status::Consistent);
}
#[test]
fn req_io_005_cash_only_header_needs_no_instruments() {
    let fixture = generate(1, 0, None).unwrap();
    let text = String::from_utf8(fixture.bytes).unwrap();
    let header = text.lines().next().unwrap();
    let mut header: qlrs_schema::Header = qlrs_io::decode(header.as_bytes()).unwrap();
    header.instruments.clear();
    let mut bytes = qlrs_io::encode(&header).unwrap();
    bytes.extend_from_slice(b"{\"seq\":1,\"ts_event\":1,\"type\":\"AccountClaim\",\"account_id\":\"A\",\"provenance\":\"observed\",\"currency\":\"INR\",\"balance_total\":\"1000000000000.00\"}\n");
    assert_eq!(report(bytes).status, Status::Consistent);
}
#[test]
fn req_io_006_escaped_strings_use_owned_cold_path_and_roundtrip() {
    let fixture = generate(2, 0, None).unwrap();
    let text = String::from_utf8(fixture.bytes)
        .unwrap()
        .replace("\"X\"", "\"\\u0058\"")
        .replace("\"20.000\"", "\"\\u00320.000\"");
    let bytes = text.into_bytes();
    let result = report(bytes.clone());
    assert_eq!(result.status, Status::Consistent);
    let parsed = qlrs_io::ParsedTrace::parse(bytes.clone()).unwrap();
    let mut written = Vec::new();
    parsed
        .write_kept(&AliveMask::all(parsed.store.units.len()), &mut written)
        .unwrap();
    assert_eq!(written, bytes);
}
#[test]
fn req_check_009_unknown_fill_never_invents_a_cash_delta() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/clean/C01-worked-example.jsonl");
    let text = std::fs::read_to_string(path).unwrap();
    let mut body = text
        .lines()
        .filter(|line| !line.contains("OrderSubmitted") && !line.contains("OrderAccepted"))
        .collect::<Vec<_>>()
        .join("\n");
    body.push('\n');
    let bytes = body.into_bytes();
    let result = report(bytes);
    assert_eq!(result.status, Status::Violation);
    assert!(result.keys().all(|key| key.invariant_id == Invariant::I3));
    assert!(result.keys().any(|key| key.sub_code == "unknown-order"));
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.reason.as_deref() == Some("missing-order-metadata"))
    );
}
#[test]
fn req_io_007_overflow_is_not_a_zero_quantity_or_precision_accusation() {
    let text = String::from_utf8(generate(1, 0, None).unwrap().bytes).unwrap();
    let huge = "99999999999999999999999999999999999999999999";
    let result = report(text.replace("20.000", huge).into_bytes());
    assert_eq!(result.status, Status::Inconclusive);
    assert!(
        result
            .findings
            .iter()
            .all(|f| f.status != Status::Unsupported)
    );
    let excess = format!("{huge}.0000000000000000001");
    assert_eq!(
        report(text.replace("20.000", &excess).into_bytes()).status,
        Status::Unsupported
    );
    let negative = format!("\"commission\":\"-{huge}\"");
    assert_eq!(
        report(
            text.replace("\"commission\":\"1.00\"", &negative)
                .into_bytes()
        )
        .status,
        Status::Unsupported
    );
}
#[test]
fn req_ledger_002_replay_is_independent_of_observed_claims() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/clean/C01-worked-example.jsonl");
    let bytes = std::fs::read_to_string(path)
        .unwrap()
        .replace("7990.00", "8090.00")
        .into_bytes();
    let mut input = qlrs_io::ParsedTrace::parse(bytes).unwrap();
    let mask = AliveMask::all(input.store.units.len());
    assert!(Ledger::replay(&input.store, &mask, None).is_err());
    Ledger::prepare(&mut input.store);
    let expected = Ledger::replay(&input.store, &mask, None).unwrap();
    assert_eq!(expected.cash, 799_000);
    assert_eq!(expected.positions.as_slice(), &[20]);
    assert_eq!(
        Checker::evaluate(&input.store, &expected).status,
        Status::Violation
    );
}
#[test]
fn req_check_010_trade_uniqueness_needs_known_instrument_scope() {
    let text = String::from_utf8(generate(2, 0, None).unwrap().bytes).unwrap();
    let mut header: qlrs_schema::Header =
        qlrs_io::decode(text.lines().next().unwrap().as_bytes()).unwrap();
    let mut other = header.instruments.first().unwrap().clone();
    other.id = "Y".into();
    header.instruments.push(other);
    let mut body = text.lines().skip(1).collect::<Vec<_>>().join("\n");
    body.push('\n');
    body = body
        .replace(
            "\"client_order_id\":\"O1\",\"instrument_id\":\"X\"",
            "\"client_order_id\":\"O1\",\"instrument_id\":\"Y\"",
        )
        .replace(
            "\"instrument_id\":\"X\",\"net_qty\":\"40.000\"",
            "\"instrument_id\":\"Y\",\"net_qty\":\"20.000\"",
        )
        .replace("\"trade_id\":\"T1\"", "\"trade_id\":\"T0\"");
    let mut known = qlrs_io::encode(&header).unwrap();
    known.extend_from_slice(body.as_bytes());
    assert_eq!(report(known).status, Status::Consistent);
    let mut unknown = qlrs_io::encode(&header).unwrap();
    for line in body
        .lines()
        .filter(|line| !line.contains("OrderSubmitted") && !line.contains("OrderAccepted"))
    {
        unknown.extend_from_slice(line.as_bytes());
        unknown.push(b'\n');
    }
    let result = report(unknown);
    assert_eq!(result.status, Status::Violation);
    assert_eq!(
        result
            .keys()
            .filter(|key| key.sub_code == "unknown-order")
            .count(),
        2
    );
    assert!(
        result
            .keys()
            .all(|key| key.sub_code != "duplicate-trade-id")
    );
}
