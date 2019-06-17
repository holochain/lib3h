use crate::{Buffer, CryptoResult};

pub trait CryptoSignature {
    fn sig_seed_size(&self) -> usize;
    fn sig_pub_size(&self) -> usize;
    fn sig_priv_size(&self) -> usize;
    fn sig_size(&self) -> usize;

    fn sig_keypair_from_seed<SeedBuffer: Buffer, PubBuffer: Buffer, PrivBuffer: Buffer>(
        &self,
        seed: &SeedBuffer,
        public_key: &mut PubBuffer,
        private_key: &mut PrivBuffer,
    ) -> CryptoResult<()>;

    fn sig_sign<PrivBuffer: Buffer, DataBuffer: Buffer, SigBuffer: Buffer>(
        &self,
        private_key: &PrivBuffer,
        data: &DataBuffer,
        signature: &mut SigBuffer,
    ) -> CryptoResult<()>;

    fn sig_verify<PubBuffer: Buffer, DataBuffer: Buffer, SigBuffer: Buffer>(
        &self,
        public_key: &PubBuffer,
        data: &DataBuffer,
        signature: &SigBuffer,
    ) -> CryptoResult<bool>;
}
