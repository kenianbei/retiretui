//! A small seeded generator of the engine's own, so a seed saved in a plan
//! draws the same markets whatever dependency versions are built against:
//! xoshiro256** seeded through splitmix64, with Box-Muller normals. The
//! promise holds on one machine; `ln` and `exp` may differ in the last
//! digit between math libraries.

use std::f64::consts::TAU;

/// One run's stream of random numbers.
pub(crate) struct Random {
    state: [u64; 4],
    spare: Option<f64>,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Random {
    /// The stream for run `run` under `seed`; every pair has its own.
    pub(crate) fn new(seed: u32, run: u32) -> Self {
        let mut mixer = (u64::from(seed) << 32) | u64::from(run);
        let state = std::array::from_fn(|_| splitmix64(&mut mixer));
        Self { state, spare: None }
    }

    fn next_u64(&mut self) -> u64 {
        let [s0, s1, s2, s3] = &mut self.state;
        let output = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let shifted = *s1 << 17;
        *s2 ^= *s0;
        *s3 ^= *s1;
        *s1 ^= *s2;
        *s0 ^= *s3;
        *s2 ^= shifted;
        *s3 = s3.rotate_left(45);
        output
    }

    /// Uniform in [0, 1).
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// Uniform over `0..count`; `count` must be positive.
    pub(crate) fn below(&mut self, count: usize) -> usize {
        let wide = u128::from(self.next_u64()) * count as u128;
        (wide >> 64) as usize
    }

    /// A standard normal draw.
    pub(crate) fn normal(&mut self) -> f64 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        let radius = (-2.0 * (1.0 - self.uniform()).ln()).sqrt();
        let angle = TAU * self.uniform();
        self.spare = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_and_run_draw_the_same_numbers_and_another_run_does_not() {
        let draws = |seed, run| {
            let mut random = Random::new(seed, run);
            (0..8)
                .map(|_| random.normal().to_bits())
                .collect::<Vec<_>>()
        };
        assert_eq!(draws(42, 3), draws(42, 3));
        assert_ne!(draws(42, 3), draws(42, 4));
        assert_ne!(draws(42, 3), draws(43, 3));
    }

    #[test]
    fn below_stays_inside_its_count() {
        let mut random = Random::new(1, 1);
        assert!((0..10_000).all(|_| random.below(7) < 7));
    }
}
