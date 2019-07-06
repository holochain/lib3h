use crate::{Buffer, CryptoResult};

#[allow(clippy::borrowed_box)]
pub trait CryptoSystem: Sync {
    fn box_clone(&self) -> Box<dyn CryptoSystem>;
    fn as_crypto_system(&self) -> &dyn CryptoSystem;

    fn buf_new_secure(&self, size: usize) -> Box<dyn Buffer>;
    fn buf_new_insecure(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(vec![0; size])
    }

    // -- random methods -- //

    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()>;

    // -- signature methods -- //

    fn sign_seed_bytes(&self) -> usize;
    fn sign_public_key_bytes(&self) -> usize;
    fn sign_secret_key_bytes(&self) -> usize;
    fn sign_bytes(&self) -> usize;

    fn sign_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;
    fn sign_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;
    fn sign(
        &self,
        signature: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        secret_key: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;
    fn sign_verify(
        &self,
        signature: &Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        public_key: &Box<dyn Buffer>,
    ) -> CryptoResult<bool>;
}

pub mod crypto_system_test;
