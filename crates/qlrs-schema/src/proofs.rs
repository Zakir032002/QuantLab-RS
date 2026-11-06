//! K06/K07 exercise the actual bitset and checked prefix implementation.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
use super::*;
#[kani::proof]
#[kani::unwind(140)]
fn alivemask_ops() {
    let length: usize = kani::any();
    kani::assume(length <= 130);
    let mut mask = AliveMask::empty(length);
    let mut expected: [bool; 130] = kani::any();
    for (index, value) in expected.iter_mut().enumerate() {
        if index < length {
            mask.set(index, *value);
        } else {
            *value = false;
        }
    }
    for _ in 0..4 {
        let index: usize = kani::any();
        kani::assume(index < 130);
        let value: bool = kani::any();
        mask.set(index, value);
        if index < length {
            expected[index] = value;
        }
    }
    for (index, value) in expected.iter().enumerate() {
        assert_eq!(mask.contains(index), *value);
    }
    assert_eq!(mask.count(), expected.iter().filter(|v| **v).count());
    let mut seen = 0;
    let mut previous = None;
    for index in mask.iter() {
        assert!(index < length && expected[index]);
        if let Some(previous) = previous {
            assert!(previous < index);
        }
        previous = Some(index);
        seen += 1;
    }
    assert_eq!(seen, expected.iter().filter(|v| **v).count());
}
#[kani::proof]
#[kani::unwind(12)]
fn prefix_sum_lemma() {
    let values: [i128; 8] = kani::any();
    let length: usize = kani::any();
    kani::assume(length <= 8);
    let prefix = checked_prefix_sums(&values[..length]);
    let fold = values[..length]
        .iter()
        .try_fold(0_i128, |sum, v| sum.checked_add(*v));
    assert_eq!(prefix.as_ref().and_then(|v| v.last()).copied(), fold);
    if let Some(prefix) = prefix {
        for end in 0..=length {
            assert_eq!(
                prefix.get(end).copied(),
                values[..end]
                    .iter()
                    .try_fold(0_i128, |sum, v| sum.checked_add(*v))
            );
        }
    }
}
