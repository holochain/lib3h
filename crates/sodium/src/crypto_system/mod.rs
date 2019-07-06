pub struct SodiumCryptoSystemConfig {}

/// the [libsodium](https://libsodium.org) ([NaCl](https://nacl.cr.yp.to/)) implementation of lib3h_crypto_api::CryptoSystem
///
/// # Examples
///
/// ```
/// extern crate lib3h_crypto_api;
/// use lib3h_crypto_api::{Buffer, CryptoSystem};
///
/// extern crate lib3h_sodium;
/// use lib3h_sodium::{
///     SecureBuffer,
///     SodiumCryptoSystem,
///     SodiumCryptoSystemConfig
/// };
///
/// // CryptoSystem is designed to be used through trait-objects like this:
/// fn test(crypto: Box<dyn CryptoSystem>) {
///     let mut seed = crypto.sec_buf_new(crypto.sign_seed_bytes());
///     crypto.randombytes_buf(&mut seed).unwrap();
///
///     let mut pubkey: Box<dyn Buffer> =
///         Box::new(vec![0; crypto.sign_public_key_bytes()]);
///     let mut seckey = crypto.sec_buf_new(crypto.sign_secret_key_bytes());
///
///     crypto.sign_seed_keypair(&seed, &mut pubkey, &mut seckey).unwrap();
///
///     let mut signature: Box<dyn Buffer> =
///         Box::new(vec![0; crypto.sign_bytes()]);
///
///     let message: Box<dyn Buffer> = Box::new(vec![1, 2, 3, 4]);
///     let bad_message: Box<dyn Buffer> = Box::new(vec![4, 3, 2, 1]);
///
///     crypto.sign(
///         &mut signature, &message, &seckey).unwrap();
///
///     assert!(crypto.sign_verify(
///         &signature, &message, &pubkey).unwrap());
///     assert!(!crypto.sign_verify(
///         &signature, &bad_message, &pubkey).unwrap());
/// }
///
/// fn main() {
///     let crypto: Box<dyn CryptoSystem> = Box::new(
///         SodiumCryptoSystem::new(SodiumCryptoSystemConfig {}));
///     test(crypto);
/// }
/// ```
pub struct SodiumCryptoSystem {
    config: SodiumCryptoSystemConfig,
}

use crate::check_init;

impl SodiumCryptoSystem {
    pub fn new(config: SodiumCryptoSystemConfig) -> Self {
        check_init();
        Self { config }
    }
}

use lib3h_crypto_api::{Buffer, CryptoError, CryptoResult};

mod secure_buffer;
pub use secure_buffer::SecureBuffer;

impl lib3h_crypto_api::CryptoSystem for SodiumCryptoSystem {
    fn sec_buf_new(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(SecureBuffer::new(size))
    }

    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()> {
        let mut buffer = buffer.write_lock();
        unsafe {
            rust_sodium_sys::randombytes_buf(raw_ptr_void!(buffer), buffer.len());
        }
        Ok(())
    }

    fn sign_seed_bytes(&self) -> usize {
        rust_sodium_sys::crypto_sign_SEEDBYTES as usize
    }
    fn sign_public_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_sign_PUBLICKEYBYTES as usize
    }
    fn sign_secret_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_sign_SECRETKEYBYTES as usize
    }
    fn sign_bytes(&self) -> usize {
        rust_sodium_sys::crypto_sign_BYTES as usize
    }

    fn sign_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if seed.len() != self.sign_seed_bytes() {
            return Err(CryptoError::BadSeedSize);
        }

        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();
        let seed = seed.read_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_seed_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
                raw_ptr_char_immut!(seed),
            );
        }

        Ok(())
    }

    fn sign_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
            );
        }

        Ok(())
    }

    fn sign(
        &self,
        signature: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        secret_key: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if signature.len() != self.sign_bytes() {
            return Err(CryptoError::BadSignatureSize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let message = message.read_lock();
        let secret_key = secret_key.read_lock();
        let mut signature = signature.write_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_detached(
                raw_ptr_char!(signature),
                std::ptr::null_mut(),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(secret_key),
            );
        }

        Ok(())
    }

    fn sign_verify(
        &self,
        signature: &Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        public_key: &Box<dyn Buffer>,
    ) -> CryptoResult<bool> {
        if signature.len() != self.sign_bytes() {
            return Err(CryptoError::BadSignatureSize);
        }

        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        Ok(unsafe {
            rust_sodium_sys::crypto_sign_verify_detached(
                raw_ptr_char_immut!(signature),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(public_key),
            )
        } == 0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use lib3h_crypto_api::{crypto_system_test, CryptoSystem};

    #[test]
    fn sodium_should_pass_crypto_system_full_suite() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new(SodiumCryptoSystemConfig {}));
        crypto_system_test::full_suite(crypto);
    }
}
