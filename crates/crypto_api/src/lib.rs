//! lib3h abstract cryptography traits and data types
//!
//! # Examples
//!
//! ```
//! extern crate lib3h_crypto_api;
//!
//! // CryptoSystem is designed to be used as a Generic Trait like this:
//! fn test<SecBuf: lib3h_crypto_api::Buffer, Crypto: lib3h_crypto_api::CryptoSystem>() {
//!     let mut seed = SecBuf::new(Crypto::SIGN_SEED_BYTES).unwrap();
//!     Crypto::randombytes_buf(&mut seed).unwrap();
//!
//!     let mut pubkey = vec![0; Crypto::SIGN_PUBLIC_KEY_BYTES];
//!     let mut seckey = SecBuf::new(Crypto::SIGN_SECRET_KEY_BYTES).unwrap();
//!
//!     Crypto::sign_seed_keypair(&seed, &mut pubkey, &mut seckey).unwrap();
//!
//!     let mut signature = vec![0; Crypto::SIGN_BYTES];
//!
//!     Crypto::sign(&mut signature, &vec![1, 2, 3, 4], &seckey).unwrap();
//!
//!     assert!(Crypto::sign_verify(&signature, &vec![1, 2, 3, 4], &pubkey).unwrap());
//!     assert!(!Crypto::sign_verify(&signature, &vec![4, 3, 2, 1], &pubkey).unwrap());
//! }
//!
//! fn main() {
//!     test::<lib3h_crypto_api::InsecureBuffer, lib3h_crypto_api::FakeCryptoSystem>();
//! }
//! ```

extern crate serde;
#[macro_use]
extern crate serde_derive;

mod error;
pub use error::{CryptoError, CryptoResult};

mod buffer;
pub use buffer::{
    insecure_buffer::InsecureBuffer, read_lock::ReadLocker, write_lock::WriteLocker, Buffer,
    BufferType, ProtectState,
};

mod random;
pub use random::CryptoRandom;

mod sign;
pub use sign::CryptoSignature;

mod system;
pub use system::CryptoSystem;

mod fake_system;
pub use fake_system::FakeCryptoSystem;
