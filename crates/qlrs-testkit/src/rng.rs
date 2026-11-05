//! Versioned SplitMix64-seeded xoshiro256** stream; no external RNG dependency.

/// Reproducible generator state, independent of architecture and thread count.
pub struct Rng([u64; 4]);
impl Rng {
    /// Expand a seed with `SplitMix64`.
    #[must_use]
    pub fn new(mut seed: u64) -> Self {
        Self(std::array::from_fn(|_| {
            seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut value = seed;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        }))
    }
    /// Advance xoshiro256** once.
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        let [mut lane0, mut lane1, mut lane2, mut lane3] = self.0;
        let result = lane1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let shifted = lane1 << 17;
        lane2 ^= lane0;
        lane3 ^= lane1;
        lane1 ^= lane2;
        lane0 ^= lane3;
        lane2 ^= shifted;
        lane3 = lane3.rotate_left(45);
        self.0 = [lane0, lane1, lane2, lane3];
        result
    }
}
