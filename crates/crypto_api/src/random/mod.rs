use crate::{Buffer, CryptoResult};

pub trait CryptoRandom {
    fn random<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()>;
}
