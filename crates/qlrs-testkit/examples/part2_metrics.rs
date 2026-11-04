//! R4 dense-violation checker measurements on the actual host.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use qlrs_schema::{AliveMask, Kind, Status};
use std::time::Instant;
fn main() {
    for orders in [2000, 20_000] {
        let fixture = qlrs_testkit::generate(orders, 42, None).unwrap();
        let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
        let mut bias = 0_i128;
        for event in 0..trace.store.len() {
            if trace.store.spine.kind.get(event) == Some(&Kind::PositionClaim) {
                let payload = *trace.store.spine.payload_idx.get(event).unwrap() as usize;
                bias += 1000;
                *trace.store.position_claims.raw.get_mut(payload).unwrap() += bias;
                *trace.store.position_claims.text.get_mut(payload).unwrap() = qlrs_schema::NONE;
            }
        }
        qlrs_ledger::Ledger::prepare(&mut trace.store);
        let alive = AliveMask::all(trace.store.units.len());
        let mut samples = Vec::new();
        let mut bytes = 0;
        let mut findings = 0;
        for _ in 0..11 {
            let start = Instant::now();
            let report = qlrs_check::Checker::check(&trace.store, &alive, None, false);
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(report.status, Status::Violation);
            assert_eq!(report.keys().count(), orders);
            samples.push(elapsed);
            findings = report.findings.len();
            bytes = qlrs_io::encode(&report).unwrap().len();
        }
        samples.sort_unstable();
        println!(
            "{{\"events\":{},\"findings\":{findings},\"report_bytes\":{bytes},\"samples_ns\":{samples:?},\"median_ns\":{}}}",
            trace.store.len(),
            samples.get(5).unwrap()
        );
    }
}
