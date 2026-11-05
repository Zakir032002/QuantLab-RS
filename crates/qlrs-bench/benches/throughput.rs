//! Repeatable Criterion timings; correctness assertions precede measurements.
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use qlrs_num::{Fixed, Rounding};
use qlrs_schema::AliveMask;
use std::hint::black_box;
fn numeric(c: &mut Criterion) {
    c.bench_function("decimal_parse", |b| {
        b.iter(|| Fixed::parse(black_box("12345678.123456789012345678"), 18).unwrap());
    });
    let qty: Fixed = "20.000".parse().unwrap();
    let px: Fixed = "100.00".parse().unwrap();
    c.bench_function("mul_rescale_i128", |b| {
        b.iter(|| {
            black_box(qty)
                .mul_rescale(black_box(px), 2, Rounding::Exact)
                .unwrap()
        });
    });
    let wide: Fixed = "10000000000000000000.000000000000000000".parse().unwrap();
    let one: Fixed = "1.000000000000000000".parse().unwrap();
    c.bench_function("mul_rescale_wide", |b| {
        b.iter(|| {
            black_box(wide)
                .mul_rescale(black_box(one), 18, Rounding::Exact)
                .unwrap()
        });
    });
}
fn pipeline(c: &mut Criterion) {
    let bytes = qlrs_testkit::generate(20_000, 42, None).unwrap().bytes;
    let mut input = qlrs_io::ParsedTrace::parse(bytes.clone()).unwrap();
    qlrs_ledger::Ledger::prepare(&mut input.store);
    let mask = AliveMask::all(input.store.units.len());
    assert_eq!(
        qlrs_check::Checker::check(&input.store, &mask, None, false).status,
        qlrs_schema::Status::Consistent
    );
    let mut group = c.benchmark_group("pipeline");
    group.throughput(Throughput::Bytes(u64::try_from(bytes.len()).unwrap()));
    group.bench_function("parse_100k", |b| {
        b.iter_batched(
            || bytes.clone(),
            |bytes| black_box(qlrs_io::ParsedTrace::parse(bytes).unwrap()),
            criterion::BatchSize::LargeInput,
        );
    });
    group.throughput(Throughput::Elements(
        u64::try_from(input.store.len()).unwrap(),
    ));
    let mut state = qlrs_ledger::Ledger::initial(&input.store, None).unwrap();
    qlrs_ledger::Ledger::replay_into(&input.store, &mask, &mut state).unwrap();
    assert_eq!(state.cash, 99_995_638_000_000);
    assert_eq!(state.positions.as_slice(), &[400_000_000]);
    group.bench_function("replay_100k", |b| {
        b.iter(|| {
            state.cash = input.store.starting_cash;
            state.positions.fill(0);
            qlrs_ledger::Ledger::replay_into(
                black_box(&input.store),
                black_box(&mask),
                black_box(&mut state),
            )
            .unwrap();
            black_box(state.cash)
        });
    });
    group.bench_function("check_100k", |b| {
        b.iter(|| {
            black_box(qlrs_check::Checker::check(
                black_box(&input.store),
                black_box(&mask),
                None,
                false,
            ))
        });
    });
    group.finish();
    let fixture =
        qlrs_testkit::generate(20_000, 42, Some((qlrs_testkit::Defect::D03, 12_000))).unwrap();
    let mut input = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_ledger::Ledger::prepare(&mut input.store);
    let digest = input.source_digest();
    c.bench_function("reduce_100k", |b| {
        b.iter(|| {
            black_box(
                qlrs_reduce::reduce(black_box(&input.store), &digest, None, fixture.target_seq)
                    .unwrap(),
            )
        });
    });
}
fn algebra(c: &mut Criterion) {
    use qlrs_reduce::FastPredicate;
    let fixture =
        qlrs_testkit::generate_shape("S9", 3, 1000, Some(qlrs_testkit::Defect::D06)).unwrap();
    let mut trace = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_ledger::Ledger::prepare(&mut trace.store);
    let original = qlrs_check::Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        None,
        true,
    );
    let key = original
        .at_seq(fixture.target_seq.unwrap())
        .unwrap()
        .instance_key
        .clone()
        .unwrap();
    let predicate = qlrs_reduce::ReductionPredicate::new(
        &trace.store,
        key.clone(),
        &original,
        None,
        &trace.source_digest(),
    )
    .unwrap();
    let mask = predicate.candidate_mask(&AliveMask::all(trace.store.units.len()));
    let table = qlrs_reduce::ContributionTable::new(&trace.store, &mask, &key).unwrap();
    let end = table.units.len();
    assert_eq!(
        table.prefix_residual(end),
        Some(key.residual_raw.parse().unwrap())
    );
    c.bench_function("prefix_residual", |b| {
        b.iter(|| black_box(black_box(&table).prefix_residual(black_box(end))));
    });
}
criterion_group!(benches, numeric, pipeline, algebra);
criterion_main!(benches);
