//! Generates tokens randomly using as secure of RNG as possible
//!

use ring::rand::SecureRandom;

pub fn generate_random_bits() -> Option<Vec<u8>> {
    let mut r = vec![0; 256];
    let rng = crate::RNG_CORE.get()?;
    rng.fill(&mut r).ok()?;
    Some(r)
}
