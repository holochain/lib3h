use crate::{Buffer, CryptoResult};

pub trait CryptoSystem {
    fn sec_buf_new(&self, size: usize) -> Box<dyn Buffer>;

    // -- random methods -- //

    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()>;

    // -- signature methods -- //

    fn sign_seed_bytes(&self) -> usize;
    fn sign_public_key_bytes(&self) -> usize;
    fn sign_secret_key_bytes(&self) -> usize;
    fn sign_bytes(&self) -> usize;

    fn sign_seed_keypair(&self, seed: &Box<dyn Buffer>, public_key: &mut Box<dyn Buffer>, secret_key: &mut Box<dyn Buffer>) -> CryptoResult<()>;
    fn sign_keypair(&self, public_key: &mut Box<dyn Buffer>, secret_key: &mut Box<dyn Buffer>) -> CryptoResult<()>;
    fn sign(&self, signature: &mut Box<dyn Buffer>, message: &Box<dyn Buffer>, secret_key: &Box<dyn Buffer>) -> CryptoResult<()>;
    fn sign_verify(&self, signature: &Box<dyn Buffer>, message: &Box<dyn Buffer>, public_key: &Box<dyn Buffer>) -> CryptoResult<bool>;
}

pub mod crypto_system_test;
