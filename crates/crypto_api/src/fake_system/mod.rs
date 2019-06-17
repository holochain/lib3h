use crate::{Buffer, CryptoRandom, CryptoResult, CryptoSignature, CryptoSystem};

pub struct FakeCryptoSystem {}

lazy_static! {
    static ref FAKE_SEED: std::sync::Arc<std::sync::Mutex<u32>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
}

const FAKE_SEQ: [u32; 4] = [0x30698bae, 0x47984c92, 0x901d24fb, 0x91fba506];

impl CryptoRandom for FakeCryptoSystem {
    fn random<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()> {
        let mut buffer = buffer.write_lock();

        let mut idx = 4;
        let mut seed = FAKE_SEED.lock().unwrap();

        for i in 0..buffer.len() {
            if idx >= FAKE_SEQ.len() {
                idx = 0;
                *seed = *seed
                    ^ std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .subsec_nanos();
            } else {
                *seed = *seed ^ FAKE_SEQ[idx];
                idx += 1;
            }
            buffer[i] = (*seed % 256) as u8;
        }

        Ok(())
    }
}

impl CryptoSignature for FakeCryptoSystem {
    #[inline]
    fn sig_seed_size(&self) -> usize {
        8
    }
    #[inline]
    fn sig_pub_size(&self) -> usize {
        8
    }
    #[inline]
    fn sig_priv_size(&self) -> usize {
        8
    }
    #[inline]
    fn sig_size(&self) -> usize {
        8
    }

    fn sig_keypair_from_seed<SeedBuffer: Buffer, PubBuffer: Buffer, PrivBuffer: Buffer>(
        &self,
        seed: &SeedBuffer,
        public_key: &mut PubBuffer,
        private_key: &mut PrivBuffer,
    ) -> CryptoResult<()> {
        public_key.write_lock().write(0, &seed.read_lock())?;
        private_key.write_lock().write(0, &seed.read_lock())?;
        Ok(())
    }

    fn sig_sign<PrivBuffer: Buffer, DataBuffer: Buffer, SigBuffer: Buffer>(
        &self,
        private_key: &PrivBuffer,
        data: &DataBuffer,
        signature: &mut SigBuffer,
    ) -> CryptoResult<()> {
        let mut signature = signature.write_lock();
        signature[0] = private_key.read_lock()[0];
        signature[1] = data.read_lock()[0];
        signature[2] = 0xff;
        signature[3] = 0xff;
        Ok(())
    }

    fn sig_verify<PubBuffer: Buffer, DataBuffer: Buffer, SigBuffer: Buffer>(
        &self,
        public_key: &PubBuffer,
        data: &DataBuffer,
        signature: &SigBuffer,
    ) -> CryptoResult<bool> {
        let public_key = public_key.read_lock();
        let data = data.read_lock();
        let signature = signature.read_lock();
        Ok(signature[0] == public_key[0]
            && signature[1] == data[0]
            && signature[2] == 0xff
            && signature[3] == 0xff)
    }
}

lazy_static! {
    static ref FAKE: FakeCryptoSystem = FakeCryptoSystem {};
}

impl CryptoSystem for FakeCryptoSystem {
    fn get() -> &'static Self {
        &FAKE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InsecureBuffer;

    #[test]
    fn it_should_randomize() {
        let mut buf1 = InsecureBuffer::new(8).unwrap();

        assert_eq!(
            "InsecureBuffer { b: [0, 0, 0, 0, 0, 0, 0, 0], p: RefCell { value: NoAccess } }",
            &format!("{:?}", buf1)
        );

        FakeCryptoSystem::get().random(&mut buf1).unwrap();

        assert_ne!(
            "InsecureBuffer { b: [0, 0, 0, 0, 0, 0, 0, 0], p: RefCell { value: NoAccess } }",
            &format!("{:?}", buf1)
        );

        let mut buf2 = InsecureBuffer::new(8).unwrap();

        FakeCryptoSystem::get().random(&mut buf2).unwrap();

        assert_ne!(&format!("{:?}", buf1), &format!("{:?}", buf2));
    }

    #[test]
    fn it_should_sign_and_verify() {
        let crypto = FakeCryptoSystem::get();

        let data = InsecureBuffer::new(8).unwrap();

        let mut seed = InsecureBuffer::new(crypto.sig_seed_size()).unwrap();
        crypto.random(&mut seed).unwrap();

        let mut pub_key = InsecureBuffer::new(crypto.sig_pub_size()).unwrap();
        let mut priv_key = InsecureBuffer::new(crypto.sig_priv_size()).unwrap();

        crypto
            .sig_keypair_from_seed(&seed, &mut pub_key, &mut priv_key)
            .unwrap();

        let mut sig = InsecureBuffer::new(crypto.sig_size()).unwrap();

        assert!(!crypto.sig_verify(&pub_key, &data, &sig).unwrap());

        crypto.sig_sign(&priv_key, &data, &mut sig).unwrap();

        assert!(crypto.sig_verify(&pub_key, &data, &sig).unwrap());
    }
}
