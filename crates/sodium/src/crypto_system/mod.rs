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
/// };
///
/// // CryptoSystem is designed to be used through trait-objects like this:
/// fn test(crypto: Box<dyn CryptoSystem>) {
///     let mut seed = crypto.buf_new_secure(crypto.sign_seed_bytes());
///     crypto.randombytes_buf(&mut seed).unwrap();
///
///     let mut pubkey: Box<dyn Buffer> =
///         Box::new(vec![0; crypto.sign_public_key_bytes()]);
///     let mut seckey = crypto.buf_new_secure(crypto.sign_secret_key_bytes());
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
///     let crypto: Box<dyn CryptoSystem> =
///         Box::new(SodiumCryptoSystem::new());
///     test(crypto);
/// }
/// ```
#[derive(Clone)]
pub struct SodiumCryptoSystem {
    pwhash_ops_limit: libc::c_ulonglong,
    pwhash_mem_limit: usize,
    pwhash_alg: libc::c_int,
}

pub const PWHASH_OPSLIMIT_INTERACTIVE: libc::c_ulonglong =
    rust_sodium_sys::crypto_pwhash_OPSLIMIT_INTERACTIVE as libc::c_ulonglong;
pub const PWHASH_OPSLIMIT_MODERATE: libc::c_ulonglong =
    rust_sodium_sys::crypto_pwhash_OPSLIMIT_MODERATE as libc::c_ulonglong;
pub const PWHASH_OPSLIMIT_SENSITIVE: libc::c_ulonglong =
    rust_sodium_sys::crypto_pwhash_OPSLIMIT_SENSITIVE as libc::c_ulonglong;

pub const PWHASH_MEMLIMIT_INTERACTIVE: usize =
    rust_sodium_sys::crypto_pwhash_MEMLIMIT_INTERACTIVE as usize;
pub const PWHASH_MEMLIMIT_MODERATE: usize =
    rust_sodium_sys::crypto_pwhash_MEMLIMIT_MODERATE as usize;
pub const PWHASH_MEMLIMIT_SENSITIVE: usize =
    rust_sodium_sys::crypto_pwhash_MEMLIMIT_SENSITIVE as usize;

pub const PWHASH_ALG_ARGON2I13: libc::c_int =
    rust_sodium_sys::crypto_pwhash_ALG_ARGON2I13 as libc::c_int;
pub const PWHASH_ALG_ARGON2ID13: libc::c_int =
    rust_sodium_sys::crypto_pwhash_ALG_ARGON2ID13 as libc::c_int;

use crate::check_init;

impl SodiumCryptoSystem {
    pub fn new() -> Self {
        check_init();
        Self {
            pwhash_ops_limit: PWHASH_OPSLIMIT_SENSITIVE,
            pwhash_mem_limit: PWHASH_MEMLIMIT_SENSITIVE,
            pwhash_alg: PWHASH_ALG_ARGON2ID13,
        }
    }

    pub fn set_pwhash_opslimit(mut self, opslimit: libc::c_ulonglong) -> Self {
        self.pwhash_ops_limit = opslimit;
        self
    }

    pub fn set_pwhash_memlimit(mut self, memlimit: usize) -> Self {
        self.pwhash_mem_limit = memlimit;
        self
    }

    pub fn set_pwhash_alg(mut self, alg: libc::c_int) -> Self {
        self.pwhash_alg = alg;
        self
    }

    pub fn set_pwhash_interactive(mut self) -> Self {
        self.set_pwhash_opslimit(PWHASH_OPSLIMIT_INTERACTIVE)
            .set_pwhash_memlimit(PWHASH_MEMLIMIT_INTERACTIVE)
    }
}

use lib3h_crypto_api::{Buffer, CryptoError, CryptoResult, CryptoSystem};

mod secure_buffer;
pub use secure_buffer::SecureBuffer;

impl CryptoSystem for SodiumCryptoSystem {
    fn box_clone(&self) -> Box<dyn CryptoSystem> {
        Box::new(self.clone())
    }

    fn as_crypto_system(&self) -> &dyn CryptoSystem {
        &*self
    }

    fn buf_new_secure(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(SecureBuffer::new(size))
    }

    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()> {
        let mut buffer = buffer.write_lock();
        unsafe {
            rust_sodium_sys::randombytes_buf(raw_ptr_void!(buffer), buffer.len());
        }
        Ok(())
    }

    fn pwhash_salt_bytes(&self) -> usize {
        rust_sodium_sys::crypto_pwhash_SALTBYTES as usize
    }
    fn pwhash_bytes(&self) -> usize {
        32
    }

    fn pwhash(
        &self,
        hash: &mut Box<dyn Buffer>,
        password: &Box<dyn Buffer>,
        salt: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if hash.len() != self.pwhash_bytes() {
            return Err(CryptoError::BadHashSize);
        }

        if salt.len() != self.pwhash_salt_bytes() {
            return Err(CryptoError::BadSaltSize);
        }

        let mut hash = hash.write_lock();
        let password = password.read_lock();
        let salt = salt.read_lock();

        let res = unsafe {
            rust_sodium_sys::crypto_pwhash(
                raw_ptr_char!(hash),
                hash.len() as libc::c_ulonglong,
                raw_ptr_ichar_immut!(password),
                password.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(salt),
                self.pwhash_ops_limit,
                self.pwhash_mem_limit,
                self.pwhash_alg,
            )
        };
        match res {
            0 => Ok(()),
            -1 => Err(CryptoError::OutOfMemory),
            _ => unreachable!(),
        }
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
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());
        crypto_system_test::full_suite(crypto);
    }
}
