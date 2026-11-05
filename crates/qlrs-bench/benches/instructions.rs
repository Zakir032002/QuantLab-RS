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
fn setup_parse() -> &'static mut Vec<u8> {
    Box::leak(Box::new(
        qlrs_testkit::generate(20_000, 42, None).unwrap().bytes,
    ))
}
#[cfg(target_os = "linux")]
#[library_benchmark]
#[bench::source(setup_parse())]
fn parse_100k(bytes: &mut [u8]) -> usize {
    std::hint::black_box(
        qlrs_io::ParsedTrace::parse(std::hint::black_box(bytes.to_vec()))
            .unwrap()
            .store
            .len(),
    )
}
#[cfg(target_os = "linux")]
struct LargeInput {
    trace: qlrs_io::ParsedTrace,
    alive: qlrs_schema::AliveMask,
    digest: String,
}
#[cfg(target_os = "linux")]
fn setup_large(defect: Option<qlrs_testkit::Defect>) -> &'static mut LargeInput {
    let fixture = qlrs_testkit::generate(20_000, 42, defect.map(|d| (d, 12_000))).unwrap();
    let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_check::Checker::prepare(&mut trace.store);
    let alive = qlrs_schema::AliveMask::all(trace.store.units.len());
    let digest = qlrs_io::digest(&trace.source);
    Box::leak(Box::new(LargeInput {
        trace,
        alive,
        digest,
    }))
}
#[cfg(target_os = "linux")]
#[library_benchmark]
#[bench::prepared(setup_large(None))]
fn check_100k(input: &mut LargeInput) -> qlrs_check::Report {
    std::hint::black_box(qlrs_check::Checker::check(
        std::hint::black_box(&input.trace.store),
        &input.alive,
        None,
        false,
    ))
}
#[cfg(target_os = "linux")]
#[library_benchmark]
#[bench::prepared(setup_large(Some(qlrs_testkit::Defect::D03)))]
fn reduce_100k(input: &mut LargeInput) -> qlrs_reduce::Reduction {
    std::hint::black_box(
        qlrs_reduce::reduce_with_limits(
            &input.trace.store,
            &input.digest,
            None,
            None,
            qlrs_reduce::ReductionLimits {
                max_calls: 1024,
                time_limit: std::time::Duration::MAX,
            },
        )
        .unwrap(),
    )
}
#[cfg(target_os = "linux")]
#[library_benchmark]
fn generate_100k() -> usize {
    let mut bytes = Vec::new();
    qlrs_testkit::generate_into(20_000, 42, None, &mut bytes).unwrap();
    std::hint::black_box(bytes).len()
}
#[cfg(target_os = "linux")]
library_benchmark_group!(name=pipeline;benchmarks=parse_100k,check_100k,reduce_100k,generate_100k);
#[cfg(target_os = "linux")]
library_benchmark_group!(name=numeric;benchmarks=decimal_parse,mul_rescale,replay_1000);
#[cfg(target_os = "linux")]
main!(library_benchmark_groups = numeric, pipeline);
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("instruction benchmark requires Linux Valgrind");
}
