use crate::{Buffer, CryptoResult};

/// Provides functions dealing with cryptographic randomness
pub trait CryptoRandom {
    /// Fill the output buffer with cryptographicly secure random bytes
    fn randombytes_buf<OutputBuffer: Buffer>(buffer: &mut OutputBuffer) -> CryptoResult<()>;
}
