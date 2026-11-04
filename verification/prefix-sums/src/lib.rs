//! Optional bounded Kani lemma, independent of the runtime crates.
#![forbid(unsafe_code)]
#[cfg(kani)]
mod proof {
    #[kani::proof]
    #[kani::unwind(6)]
    fn shared_prefix_union_matches_direct_sum() {
        let input: [i64; 4] = kani::any();
        let closures: [u8; 4] = kani::any();
        let mut seen = 0_u8;
        let mut prefix = 0_i128;
        for closure in closures {
            for (index, value) in input.iter().enumerate() {
                let bit = 1_u8 << index;
                if closure & bit != 0 && seen & bit == 0 {
                    prefix = prefix.checked_add(i128::from(*value)).unwrap();
                    seen |= bit;
                }
            }
            let mut direct = 0_i128;
            for (index, value) in input.iter().enumerate() {
                if seen & (1_u8 << index) != 0 {
                    direct = direct.checked_add(i128::from(*value)).unwrap();
                }
            }
            assert_eq!(prefix, direct);
        }
    }
}
