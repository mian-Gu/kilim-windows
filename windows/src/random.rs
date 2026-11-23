//! The same SplitMix64 sequence used by the upstream procedural rugs and test scenes.
pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_splitmix_values() {
        let mut random = Rng::new(42);
        assert!((random.next() - 0.7415648787718233).abs() < 1e-15);
        assert!((random.next() - 0.1599103928769201).abs() < 1e-15);
    }
}
