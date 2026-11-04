//! Deterministic incremental 128-bit tabulation, with exact collision checking.
#[derive(Clone, Default)]
pub(crate) struct HashState {
    units: Vec<u32>,
    hash: [u64; 2],
}
fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
impl HashState {
    fn toggle(&mut self, unit: u32) {
        if let Some(low) = self.hash.first_mut() {
            *low ^= mix(u64::from(unit));
        }
        if let Some(high) = self.hash.get_mut(1) {
            *high ^= mix(u64::from(unit) ^ 0xd1b5_4a32_d192_ed03);
        }
    }
    pub fn update(&mut self, units: &[u32]) -> [u64; 2] {
        // Toggle only changed IDs. Fixed-window bounds are part of predicate identity.
        let previous = std::mem::take(&mut self.units);
        let mut a = previous.iter().peekable();
        let mut b = units.iter().peekable();
        loop {
            match (a.peek(), b.peek()) {
                (Some(left), Some(right)) if left == right => {
                    a.next();
                    b.next();
                }
                (Some(left), Some(right)) if left < right => {
                    self.toggle(**left);
                    a.next();
                }
                (_, Some(right)) => {
                    self.toggle(**right);
                    b.next();
                }
                (Some(left), None) => {
                    self.toggle(**left);
                    a.next();
                }
                (None, None) => break,
            }
        }
        self.units = units.to_vec();
        self.hash
    }
}
#[derive(Clone)]
pub(crate) struct Cached<T> {
    pub units: Vec<u32>,
    pub value: T,
}
pub(crate) fn lookup<'a, T>(bucket: Option<&'a Vec<Cached<T>>>, units: &[u32]) -> Option<&'a T> {
    bucket?
        .iter()
        .find(|entry| entry.units == units)
        .map(|entry| &entry.value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn req_reduce_011_cache_forced_collision_and_incremental_toggles() {
        let mut state = HashState::default();
        let first = state.update(&[1, 3, 5]);
        assert_ne!(first, state.update(&[1, 5]));
        assert_eq!(first, state.update(&[1, 3, 5]));
        let collision_bucket = vec![
            Cached {
                units: vec![1, 3],
                value: true,
            },
            Cached {
                units: vec![2, 4],
                value: false,
            },
        ];
        assert_eq!(lookup(Some(&collision_bucket), &[1, 3]), Some(&true));
        assert_eq!(lookup(Some(&collision_bucket), &[2, 4]), Some(&false));
        assert_eq!(lookup(Some(&collision_bucket), &[1, 4]), None);
    }
}
