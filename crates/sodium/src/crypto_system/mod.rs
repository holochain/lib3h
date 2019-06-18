pub mod secure_buffer;
pub use secure_buffer::SecureBuffer;

pub struct SodiumCryptoSystem {}

mod random;
mod sign;

lazy_static! {
    static ref SODIUM_CRYPTO_SYSTEM: SodiumCryptoSystem = SodiumCryptoSystem {};
}

impl lib3h_crypto_api::CryptoSystem for SodiumCryptoSystem {
    fn get() -> &'static Self {
        &SODIUM_CRYPTO_SYSTEM
    }
}
