/// a really simple seeded unit prng
/// uses the Xoroshiro128+ prng under the hood
/// http://prng.di.unimi.it/
use xoroshiro128::{Rng, SeedableRng, Xoroshiro128Rng};

pub struct SeededUnitPrng {
    pub seed: [u64; 2],
    prng: Xoroshiro128Rng,
}

impl From<[u64; 2]> for SeededUnitPrng {
    fn from(seed: [u64; 2]) -> Self {
        Self {
            seed,
            prng: Xoroshiro128Rng::from_seed(seed),
        }
    }
}

impl Iterator for SeededUnitPrng {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        // http://xoroshiro.di.unimi.it/
        // A standard double (64-bit) floating-point number in IEEE floating point
        // format has 52 bits of mantissa, plus an implicit one at the left of the
        // mantissa. Thus, even if there are 52 bits of mantissa, the representation can
        // actually store numbers with 53 significant binary digits.
        //
        // Because of this fact, in C99 a 64-bit unsigned integer x should be converted
        // to a 64-bit double using the expression
        //
        //   #include <stdint.h>
        //   (x >> 11) * (1. / (UINT64_C(1) << 53))])
        //
        // In Java, the same result can be obtained with
        //
        //   (x >>> 11) * 0x1.0p-53)
        //
        // This conversion guarantees that all dyadic rationals of the form k / 2−53
        // will be equally likely. Note that this conversion prefers the high bits of x,
        // but you can alternatively use the lowest bits.)
        Some((self.prng.gen::<u64>() >> 11) as f64 * hexf64!("0x1.0p-53"))
    }
}

pub struct SeededBooleanPrng {
    pub seed: [u64; 2],
    prng: Xoroshiro128Rng,
}

impl From<[u64; 2]> for SeededBooleanPrng {
    fn from(seed: [u64; 2]) -> Self {
        Self {
            seed,
            prng: Xoroshiro128Rng::from_seed(seed),
        }
    }
}

impl Iterator for SeededBooleanPrng {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        // https://en.wikipedia.org/wiki/Xoroshiro128%2B
        // Thus, programmers should prefer the highest bits (e.g., making a heads/tails by writing
        // random_number < 0 rather than random_number & 1).
        Some(i64::from_be_bytes(self.prng.gen::<u64>().to_be_bytes()).is_positive())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_unit_prng() {
        let some_seed = [98279182398273, 19287391287398273];

        let mut some_unit_prng = SeededUnitPrng::from(some_seed);

        assert_eq!(some_unit_prng.next(), Some(0.001050899301922037));
        assert_eq!(some_unit_prng.next(), Some(0.4424052192261052));
        assert_eq!(some_unit_prng.next(), Some(0.41113251913353943));
        assert_eq!(some_unit_prng.next(), Some(0.7822720241572837));
        assert_eq!(some_unit_prng.next(), Some(0.9282316932314376));

        let some_seed = [09428033324987394823, 1209830827328329387];

        let mut some_unit_prng = SeededUnitPrng::from(some_seed);

        assert_eq!(some_unit_prng.next(), Some(0.5766797712273188));
        assert_eq!(some_unit_prng.next(), Some(0.32086035649929934));
        assert_eq!(some_unit_prng.next(), Some(0.02371491442257112));
        assert_eq!(some_unit_prng.next(), Some(0.30355498178878104));
        assert_eq!(some_unit_prng.next(), Some(0.667949175296175));
    }

    #[test]
    pub fn test_boolean_prng() {
        let some_seed = [98279182398273, 19287391287398273];

        let mut some_unit_prng = SeededBooleanPrng::from(some_seed);

        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(false));
        assert_eq!(some_unit_prng.next(), Some(false));

        let some_seed = [09428033324987394823, 1209830827328329387];

        let mut some_unit_prng = SeededBooleanPrng::from(some_seed);

        assert_eq!(some_unit_prng.next(), Some(false));
        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(true));
        assert_eq!(some_unit_prng.next(), Some(false));
    }

}
