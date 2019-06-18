use crate::{Buffer, CryptoResult};

pub trait CryptoRandom {
    fn randombytes_buf<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()>;
}
