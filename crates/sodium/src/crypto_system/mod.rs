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

    fn hash_sha256_bytes(&self) -> usize {
        rust_sodium_sys::crypto_hash_sha256_BYTES as usize
    }
    fn hash_sha512_bytes(&self) -> usize {
        rust_sodium_sys::crypto_hash_sha512_BYTES as usize
    }

    fn hash_sha256(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()> {
        if hash.len() != self.hash_sha256_bytes() {
            return Err(CryptoError::BadHashSize);
        }

        unsafe {
            let mut hash = hash.write_lock();
            let data = data.read_lock();
            if rust_sodium_sys::crypto_hash_sha256(
                raw_ptr_char!(hash),
                raw_ptr_char_immut!(data),
                data.len() as libc::c_ulonglong,
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn hash_sha512(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()> {
        if hash.len() != self.hash_sha512_bytes() {
            return Err(CryptoError::BadHashSize);
        }

        unsafe {
            let mut hash = hash.write_lock();
            let data = data.read_lock();
            rust_sodium_sys::crypto_hash_sha512(
                raw_ptr_char!(hash),
                raw_ptr_char_immut!(data),
                data.len() as libc::c_ulonglong,
            );
        }

        Ok(())
    }

    fn generic_hash_min_bytes(&self) -> usize {
        rust_sodium_sys::crypto_generichash_BYTES_MIN as usize
    }

    fn generic_hash_max_bytes(&self) -> usize {
        rust_sodium_sys::crypto_generichash_BYTES_MAX as usize
    }

    fn generic_hash_key_min_bytes(&self) -> usize {
        rust_sodium_sys::crypto_generichash_KEYBYTES_MIN as usize
    }

    fn generic_hash_key_max_bytes(&self) -> usize {
        rust_sodium_sys::crypto_generichash_KEYBYTES_MAX as usize
    }

    fn generic_hash(
        &self,
        hash: &mut Box<dyn Buffer>,
        data: &Box<dyn Buffer>,
        key: Option<&Box<dyn Buffer>>,
    ) -> CryptoResult<()> {
        if hash.len() < self.generic_hash_min_bytes() || hash.len() > self.generic_hash_max_bytes()
        {
            return Err(CryptoError::BadHashSize);
        }

        if key.is_some()
            && (key.unwrap().len() < self.generic_hash_key_min_bytes()
                || key.unwrap().len() > self.generic_hash_key_max_bytes())
        {
            return Err(CryptoError::BadKeySize);
        }

        let my_key_locker;
        let mut my_key = std::ptr::null();
        let mut my_key_len = 0 as usize;
        if let Some(key) = key {
            my_key_locker = key.read_lock();
            my_key = raw_ptr_char_immut!(my_key_locker);
            my_key_len = my_key_locker.len() as usize;
        }

        unsafe {
            let mut hash = hash.write_lock();
            let data = data.read_lock();
            rust_sodium_sys::crypto_generichash(
                raw_ptr_char!(hash),
                hash.len() as usize,
                raw_ptr_char_immut!(data),
                data.len() as libc::c_ulonglong,
                my_key,
                my_key_len,
            );
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

    fn kdf_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kdf_KEYBYTES as usize
    }

    fn kdf_context_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kdf_CONTEXTBYTES as usize
    }

    fn kdf_min_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kdf_BYTES_MIN as usize
    }

    fn kdf_max_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kdf_BYTES_MAX as usize
    }

    fn kdf(
        &self,
        out_buffer: &mut Box<dyn Buffer>,
        index: u64,
        context: &Box<dyn Buffer>,
        parent: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if out_buffer.len() < self.kdf_min_bytes() || out_buffer.len() > self.kdf_max_bytes() {
            return Err(CryptoError::BadOutBufferSize);
        }

        if parent.len() != self.kdf_key_bytes() {
            return Err(CryptoError::BadParentSize);
        }

        if context.len() != self.kdf_context_bytes() {
            return Err(CryptoError::BadContextSize);
        }

        let mut out_buffer = out_buffer.write_lock();
        let context = context.read_lock();
        let parent = parent.read_lock();

        unsafe {
            if rust_sodium_sys::crypto_kdf_derive_from_key(
                raw_ptr_char!(out_buffer),
                out_buffer.len(),
                index,
                raw_ptr_ichar_immut!(context),
                raw_ptr_char_immut!(parent),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
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
            if rust_sodium_sys::crypto_sign_seed_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
                raw_ptr_char_immut!(seed),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
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
            if rust_sodium_sys::crypto_sign_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
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
            if rust_sodium_sys::crypto_sign_detached(
                raw_ptr_char!(signature),
                std::ptr::null_mut(),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(secret_key),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
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
        } == 0 as libc::c_int)
    }

    fn kx_seed_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kx_SEEDBYTES as usize
    }
    fn kx_public_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kx_PUBLICKEYBYTES as usize
    }
    fn kx_secret_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kx_SECRETKEYBYTES as usize
    }
    fn kx_session_key_bytes(&self) -> usize {
        rust_sodium_sys::crypto_kx_SESSIONKEYBYTES as usize
    }

    fn kx_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if seed.len() != self.kx_seed_bytes() {
            return Err(CryptoError::BadSeedSize);
        }

        if public_key.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.kx_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();
        let seed = seed.read_lock();

        unsafe {
            if rust_sodium_sys::crypto_kx_seed_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
                raw_ptr_char_immut!(seed),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn kx_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if public_key.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.kx_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();

        unsafe {
            if rust_sodium_sys::crypto_kx_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn kx_client_session_keys(
        &self,
        client_rx: &mut Box<dyn Buffer>,
        client_tx: &mut Box<dyn Buffer>,
        client_pk: &Box<dyn Buffer>,
        client_sk: &Box<dyn Buffer>,
        server_pk: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if client_rx.len() != self.kx_session_key_bytes() {
            return Err(CryptoError::BadRxSessionKeySize);
        }

        if client_tx.len() != self.kx_session_key_bytes() {
            return Err(CryptoError::BadTxSessionKeySize);
        }

        if client_pk.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if client_sk.len() != self.kx_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        if server_pk.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        unsafe {
            let mut client_rx = client_rx.write_lock();
            let mut client_tx = client_tx.write_lock();
            let client_pk = client_pk.read_lock();
            let client_sk = client_sk.read_lock();
            let server_pk = server_pk.read_lock();
            if rust_sodium_sys::crypto_kx_client_session_keys(
                raw_ptr_char!(client_rx),
                raw_ptr_char!(client_tx),
                raw_ptr_char_immut!(client_pk),
                raw_ptr_char_immut!(client_sk),
                raw_ptr_char_immut!(server_pk),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn kx_server_session_keys(
        &self,
        server_rx: &mut Box<dyn Buffer>,
        server_tx: &mut Box<dyn Buffer>,
        server_pk: &Box<dyn Buffer>,
        server_sk: &Box<dyn Buffer>,
        client_pk: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if server_rx.len() != self.kx_session_key_bytes() {
            return Err(CryptoError::BadRxSessionKeySize);
        }

        if server_tx.len() != self.kx_session_key_bytes() {
            return Err(CryptoError::BadTxSessionKeySize);
        }

        if server_pk.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if server_sk.len() != self.kx_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        if client_pk.len() != self.kx_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        unsafe {
            let mut server_rx = server_rx.write_lock();
            let mut server_tx = server_tx.write_lock();
            let server_pk = server_pk.read_lock();
            let server_sk = server_sk.read_lock();
            let client_pk = client_pk.read_lock();
            if rust_sodium_sys::crypto_kx_server_session_keys(
                raw_ptr_char!(server_rx),
                raw_ptr_char!(server_tx),
                raw_ptr_char_immut!(server_pk),
                raw_ptr_char_immut!(server_sk),
                raw_ptr_char_immut!(client_pk),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn aead_nonce_bytes(&self) -> usize {
        rust_sodium_sys::crypto_aead_xchacha20poly1305_ietf_NPUBBYTES as usize
    }

    fn aead_auth_bytes(&self) -> usize {
        rust_sodium_sys::crypto_aead_xchacha20poly1305_ietf_ABYTES as usize
    }

    fn aead_secret_bytes(&self) -> usize {
        rust_sodium_sys::crypto_aead_xchacha20poly1305_ietf_KEYBYTES as usize
    }

    fn aead_encrypt(
        &self,
        cipher: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        adata: Option<&Box<dyn Buffer>>,
        nonce: &Box<dyn Buffer>,
        secret: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if cipher.len() != message.len() + self.aead_auth_bytes() {
            return Err(CryptoError::BadCipherSize);
        }

        if nonce.len() != self.aead_nonce_bytes() {
            return Err(CryptoError::BadNonceSize);
        }

        if secret.len() != self.aead_secret_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let my_adata_locker;
        let mut my_adata = std::ptr::null();
        let mut my_ad_len = 0 as libc::c_ulonglong;
        if let Some(adata) = adata {
            my_adata_locker = adata.read_lock();
            my_adata = raw_ptr_char_immut!(my_adata_locker);
            my_ad_len = my_adata_locker.len() as libc::c_ulonglong;
        }

        let mut cipher = cipher.write_lock();
        let message = message.read_lock();
        let nonce = nonce.read_lock();
        let secret = secret.read_lock();

        unsafe {
            if rust_sodium_sys::crypto_aead_xchacha20poly1305_ietf_encrypt(
                raw_ptr_char!(cipher),
                std::ptr::null_mut(),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                my_adata,
                my_ad_len,
                std::ptr::null_mut(),
                raw_ptr_char_immut!(nonce),
                raw_ptr_char_immut!(secret),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::Generic("libsodium fail".to_string()));
            }
        }

        Ok(())
    }

    fn aead_decrypt(
        &self,
        message: &mut Box<dyn Buffer>,
        cipher: &Box<dyn Buffer>,
        adata: Option<&Box<dyn Buffer>>,
        nonce: &Box<dyn Buffer>,
        secret: &Box<dyn Buffer>,
    ) -> CryptoResult<()> {
        if message.len() != cipher.len() - self.aead_auth_bytes() {
            return Err(CryptoError::BadMessageSize);
        }

        if nonce.len() != self.aead_nonce_bytes() {
            return Err(CryptoError::BadNonceSize);
        }

        if secret.len() != self.aead_secret_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let my_adata_locker;
        let mut my_adata = std::ptr::null();
        let mut my_ad_len = 0 as libc::c_ulonglong;

        if let Some(adata) = adata {
            my_adata_locker = adata.read_lock();
            my_adata = raw_ptr_char_immut!(my_adata_locker);
            my_ad_len = my_adata_locker.len() as libc::c_ulonglong;
        }

        let mut message = message.write_lock();
        let cipher = cipher.read_lock();
        let nonce = nonce.read_lock();
        let secret = secret.read_lock();

        unsafe {
            if rust_sodium_sys::crypto_aead_xchacha20poly1305_ietf_decrypt(
                raw_ptr_char!(message),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                raw_ptr_char_immut!(cipher),
                cipher.len() as libc::c_ulonglong,
                my_adata,
                my_ad_len,
                raw_ptr_char_immut!(nonce),
                raw_ptr_char_immut!(secret),
            ) != 0 as libc::c_int
            {
                return Err(CryptoError::CouldNotDecrypt);
            }
        }

        Ok(())
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

    #[test]
    fn sodium_should_kdf_derive_as_expected() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());

        let ctx1: Box<dyn Buffer> = Box::new(vec![1; crypto.kdf_context_bytes()]);
        let ctx2: Box<dyn Buffer> = Box::new(vec![2; crypto.kdf_context_bytes()]);

        let root: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_key_bytes()]);
        let mut a_1_1: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);
        let mut a_2_1: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);
        let mut a_1_2: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);
        let mut b_1_1: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);
        let mut b_2_1: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);
        let mut b_1_2: Box<dyn Buffer> = Box::new(vec![0; crypto.kdf_min_bytes()]);

        crypto.kdf(&mut a_1_1, 1, &ctx1, &root).unwrap();
        crypto.kdf(&mut a_2_1, 2, &ctx1, &root).unwrap();
        crypto.kdf(&mut a_1_2, 1, &ctx2, &root).unwrap();

        crypto.kdf(&mut b_1_1, 1, &ctx1, &root).unwrap();
        crypto.kdf(&mut b_2_1, 2, &ctx1, &root).unwrap();
        crypto.kdf(&mut b_1_2, 1, &ctx2, &root).unwrap();

        assert_eq!(
            "[163, 55, 238, 63, 149, 30, 99, 242, 9, 249, 55, 237, 48, 207, 230, 249]",
            format!("{:?}", &*a_1_1.read_lock()),
            "a_1_1 exact"
        );
        assert_eq!(
            "[89, 155, 201, 255, 133, 74, 112, 143, 164, 90, 72, 218, 209, 152, 4, 103]",
            format!("{:?}", &*a_2_1.read_lock()),
            "a_2_1 exact"
        );
        assert_eq!(
            "[138, 140, 25, 65, 64, 127, 136, 237, 195, 38, 209, 228, 17, 110, 221, 107]",
            format!("{:?}", &*a_1_2.read_lock()),
            "a_1_2 exact"
        );

        assert_eq!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", b_1_1),
            "a_1_1 == b_1_1"
        );
        assert_eq!(
            &format!("{:?}", a_2_1),
            &format!("{:?}", b_2_1),
            "a_2_1 == b_2_1"
        );
        assert_eq!(
            &format!("{:?}", a_1_2),
            &format!("{:?}", b_1_2),
            "a_1_2 == b_1_2"
        );

        assert_ne!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", a_2_1),
            "a_1_1 != a_2_1"
        );
        assert_ne!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", a_1_2),
            "a_1_1 != a_1_2"
        );
        assert_ne!(
            &format!("{:?}", a_2_1),
            &format!("{:?}", a_1_2),
            "a_2_1 != a_1_2"
        );
    }
}
