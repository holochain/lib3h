mod secure_buffer;
pub use secure_buffer::SecureBuffer;

/// the [libsodium](https://libsodium.org) ([NaCl](https://nacl.cr.yp.to/)) implementation of lib3h_crypto_api::CryptoSystem
///
/// # Examples
///
/// ```
/// extern crate lib3h_crypto_api;
/// use lib3h_crypto_api::Buffer;
///
/// extern crate lib3h_sodium;
/// use lib3h_sodium::SecureBuffer;
///
/// // It is recommended to use CryptoSystem as a Generic Trait like this:
/// fn test<SecBuf: lib3h_crypto_api::Buffer, Crypto: lib3h_crypto_api::CryptoSystem>() {
///     let mut seed = SecBuf::new(Crypto::SIGN_SEED_BYTES).unwrap();
///     Crypto::randombytes_buf(&mut seed).unwrap();
///
///     let mut pubkey = vec![0; Crypto::SIGN_PUBLIC_KEY_BYTES];
///     let mut seckey = SecBuf::new(Crypto::SIGN_SECRET_KEY_BYTES).unwrap();
///
///     Crypto::sign_seed_keypair(&seed, &mut pubkey, &mut seckey).unwrap();
///
///     let mut signature = vec![0; Crypto::SIGN_BYTES];
///
///     Crypto::sign(&mut signature, &vec![1, 2, 3, 4], &seckey).unwrap();
///
///     assert!(Crypto::sign_verify(&signature, &vec![1, 2, 3, 4], &pubkey).unwrap());
///     assert!(!Crypto::sign_verify(&signature, &vec![4, 3, 2, 1], &pubkey).unwrap());
/// }
///
/// fn main() {
///     test::<lib3h_sodium::SecureBuffer, lib3h_sodium::SodiumCryptoSystem>();
/// }
/// ```
pub struct SodiumCryptoSystem {}

mod random;
mod sign;

use lib3h_crypto_api::Buffer;

impl lib3h_crypto_api::CryptoSystem for SodiumCryptoSystem {}
