use lib3h_crypto_api::{Buffer, CryptoRandom, CryptoResult};

use crate::{check_init, crypto_system::SodiumCryptoSystem};

impl CryptoRandom for SodiumCryptoSystem {
    fn randombytes_buf<OutputBuffer: Buffer>(buffer: &mut OutputBuffer) -> CryptoResult<()> {
        check_init();
        let mut buffer = buffer.write_lock();
        unsafe {
            rust_sodium_sys::randombytes_buf(raw_ptr_void!(buffer), buffer.len());
        }
        Ok(())
    }
}
