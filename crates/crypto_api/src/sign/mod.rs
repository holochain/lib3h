use crate::{Buffer, CryptoResult};

/// Provides functions dealing with cryptographic / digital signatures
pub trait CryptoSignature {
    /// byte length of seed for generating signature keypairs
    const SIGN_SEED_BYTES: usize;

    /// byte length of signature public keys
    const SIGN_PUBLIC_KEY_BYTES: usize;

    /// byte length of signature secret keys
    const SIGN_SECRET_KEY_BYTES: usize;

    /// byte length of signatures
    const SIGN_BYTES: usize;

    /// Given a seed buffer of SIGN_SEED_BYTES length,
    /// produce a public key of SIGN_PUBLIC_KEY_BYTES length,
    /// and an associated secret key of SIGN_SECRET_KEY_BYTES length.
    fn sign_seed_keypair<SeedBuffer: Buffer, PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        seed: &SeedBuffer,
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()>;

    /// Produce a public key of SIGN_PUBLIC_KEY_BYTES length,
    /// and an associated secret key of SIGN_SECRET_KEY_BYTES length.
    fn sign_keypair<PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()>;

    /// Generate a cryptographic / digital signature for a message with the
    /// given secret key.
    /// The signature bytes are placed in the `signature` parameter.
    fn sign<SignatureBuffer: Buffer, MessageBuffer: Buffer, SecretKeyBuffer: Buffer>(
        signature: &mut SignatureBuffer,
        message: &MessageBuffer,
        secret_key: &SecretKeyBuffer,
    ) -> CryptoResult<()>;

    /// Given a public key, verify that `signature` was generated for
    /// the supplied message data with the associated secret key.
    fn sign_verify<SignatureBuffer: Buffer, MessageBuffer: Buffer, PublicKeyBuffer: Buffer>(
        signature: &SignatureBuffer,
        message: &MessageBuffer,
        public_key: &PublicKeyBuffer,
    ) -> CryptoResult<bool>;
}
