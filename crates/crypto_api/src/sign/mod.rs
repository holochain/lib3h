use crate::{Buffer, CryptoResult};

pub trait CryptoSignature {
    fn sign_seed_bytes(&self) -> usize;
    fn sign_public_key_bytes(&self) -> usize;
    fn sign_secret_key_bytes(&self) -> usize;
    fn sign_bytes(&self) -> usize;

    fn sign_seed_keypair<SeedBuffer: Buffer, PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        &self,
        seed: &SeedBuffer,
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()>;

    fn sign<SignatureBuffer: Buffer, MessageBuffer: Buffer, SecretKeyBuffer: Buffer>(
        &self,
        signature: &mut SignatureBuffer,
        message: &MessageBuffer,
        secret_key: &SecretKeyBuffer,
    ) -> CryptoResult<()>;

    fn sign_verify<SignatureBuffer: Buffer, MessageBuffer: Buffer, PublicKeyBuffer: Buffer>(
        &self,
        signature: &SignatureBuffer,
        message: &MessageBuffer,
        public_key: &PublicKeyBuffer,
    ) -> CryptoResult<bool>;
}
