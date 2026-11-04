//! Linux instruction-count regression workload.
#![forbid(unsafe_code)]
// The benchmark macros generate public harness symbols without documentation.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#[cfg(target_os = "linux")]
use gungraun::{library_benchmark, library_benchmark_group, main};
#[cfg(target_os = "linux")]
#[library_benchmark]
fn decimal_parse() -> qlrs_num::Fixed {
    std::hint::black_box(
        qlrs_num::Fixed::parse(std::hint::black_box("12345678.123456789012345678"), 18).unwrap(),
    )
}
#[cfg(target_os = "linux")]
#[library_benchmark]
fn mul_rescale() -> qlrs_num::Fixed {
    let q = qlrs_num::Fixed::new(std::hint::black_box(20_000), 3).unwrap();
    let p = qlrs_num::Fixed::new(std::hint::black_box(10_000), 2).unwrap();
    std::hint::black_box(q.mul_rescale(p, 2, qlrs_num::Rounding::Exact).unwrap())
}
#[cfg(target_os = "linux")]
struct ReplayInput {
    store: qlrs_schema::TraceStore,
    alive: qlrs_schema::AliveMask,
    state: qlrs_ledger::ExpectedState,
}
#[cfg(target_os = "linux")]
fn setup_replay() -> &'static mut ReplayInput {
    let fixture = qlrs_testkit::generate(200, 42, None).unwrap();
    let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_ledger::Ledger::prepare(&mut trace.store);
    let alive = qlrs_schema::AliveMask::all(trace.store.units.len());
    let expected = qlrs_ledger::Ledger::replay(&trace.store, &alive, None).unwrap();
    assert_eq!(expected.cash, 99_999_956_380_000);
    assert_eq!(expected.positions.as_slice(), &[4_000_000]);
    let state = qlrs_ledger::Ledger::initial(&trace.store, None).unwrap();
    // One bounded, process-lifetime allocation outside the measured region;
    // borrowing it avoids measuring destructor/allocator instruction noise.
    Box::leak(Box::new(ReplayInput {
        store: trace.store,
        alive,
        state,
    }))
}
#[cfg(target_os = "linux")]
#[library_benchmark]
#[bench::prepared(setup_replay())]
fn replay_1000(input: &mut ReplayInput) -> i128 {
    input.state.cash = input.store.starting_cash;
    input.state.positions.fill(0);
    qlrs_ledger::Ledger::replay_into(
        std::hint::black_box(&input.store),
        std::hint::black_box(&input.alive),
        std::hint::black_box(&mut input.state),
    )
    .unwrap();
    std::hint::black_box(input.state.cash)
}
#[cfg(target_os = "linux")]
library_benchmark_group!(name=numeric;benchmarks=decimal_parse,mul_rescale,replay_1000);
#[cfg(target_os = "linux")]
main!(library_benchmark_groups = numeric);
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("instruction benchmark requires Linux Valgrind");
}
