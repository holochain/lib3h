use crate::{Buffer, CryptoResult};

/// A trait describing a cryptographic system implementation compatible
/// with Lib3h and Holochain.
#[allow(clippy::borrowed_box)]
pub trait CryptoSystem: Sync {
    /// Crypto System is designed to be used as a trait-object
    /// Since we can't get a sized clone, provide clone in a Box.
    fn box_clone(&self) -> Box<dyn CryptoSystem>;

    /// helps work around some sizing issues with rust trait-objects
    fn as_crypto_system(&self) -> &dyn CryptoSystem;

    /// create a new memory secured buffer
    /// that is compatible with this crypto system
    fn buf_new_secure(&self, size: usize) -> Box<dyn Buffer>;

    /// this is just a helper to create a
    /// sized boxed Vec<u8> as a Box<dyn Buffer>
    fn buf_new_insecure(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(vec![0; size])
    }

    // -- random methods -- //

    /// fill all the bytes in the buffer with secure random data
    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()>;

    // -- hash methods -- //

    /// bytelength of sha256 hash
    fn hash_sha256_bytes(&self) -> usize;

    /// bytelength of sha512 hash
    fn hash_sha512_bytes(&self) -> usize;

    /// bytelength of pwhash salt
    fn pwhash_salt_bytes(&self) -> usize;

    /// bytelength of pwhash
    fn pwhash_bytes(&self) -> usize;

    /// run a cpu/memory intensive password hash against password / salt
    fn pwhash(
        &self,
        hash: &mut Box<dyn Buffer>,
        password: &Box<dyn Buffer>,
        salt: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// compute a sha256 hash for `data`, storing it in `hash`
    fn hash_sha256(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()>;

    /// compute a sha512 hash for `data`, storing it in `hash`
    fn hash_sha512(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()>;

    // -- signature methods -- //

    /// bytelength of signature seed
    fn sign_seed_bytes(&self) -> usize;

    /// bytelength of signature public key
    fn sign_public_key_bytes(&self) -> usize;

    /// bytelength of signature secret key
    fn sign_secret_key_bytes(&self) -> usize;

    /// bytelength of a digital signature
    fn sign_bytes(&self) -> usize;

    /// generate a deterministic signature public / secret keypair
    /// based off the given seed entropy
    fn sign_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate a pure entropy based signature public / secret keypair
    fn sign_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate a digital signature for `message` with the given secret key
    fn sign(
        &self,
        signature: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        secret_key: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// verify that the digital `signature` is valid for given `message` and
    /// `public_key`
    fn sign_verify(
        &self,
        signature: &Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        public_key: &Box<dyn Buffer>,
    ) -> CryptoResult<bool>;
}

pub mod crypto_system_test;
