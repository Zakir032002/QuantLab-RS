//! Isolated allocation instrumentation exception: ADR-0005; covered by Miri.
#![deny(unsafe_code)]
#![allow(clippy::unwrap_used)]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<usize>=const{Cell::new(0)};}
struct Counting;
fn record() {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = COUNT.try_with(|count| count.set(count.get().saturating_add(1)));
        }
    });
}
#[allow(unsafe_code)]
// SAFETY: this allocator forwards every pointer and original layout unchanged
// to System; thread-local counting never owns or dereferences the allocation.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
#[test]
fn req_ledger_001_zero_allocations_in_replay_loop() {
    let fixture = qlrs_testkit::generate(20, 0, None).unwrap();
    let mut input = qlrs_io::ParsedTrace::parse(fixture.bytes).unwrap();
    qlrs_ledger::Ledger::prepare(&mut input.store);
    let mask = qlrs_schema::AliveMask::all(input.store.units.len());
    let mut state = qlrs_ledger::Ledger::initial(&input.store, None).unwrap();
    COUNT.with(|count| count.set(0));
    ACTIVE.with(|active| active.set(true));
    let mut success = true;
    for _ in 0..100 {
        state.cash = input.store.starting_cash;
        state.positions.fill(0);
        success &= qlrs_ledger::Ledger::replay_into(&input.store, &mask, &mut state).is_ok();
    }
    ACTIVE.with(|active| active.set(false));
    let count = COUNT.with(Cell::get);
    assert!(success);
    assert_eq!(state.cash, 99_999_995_998_000);
    assert_eq!(state.positions.as_slice(), &[400_000]);
    assert_eq!(count, 0, "replay allocated {count} times");
}
