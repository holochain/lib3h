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
    fn randombytes_buf<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()> {
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
    fn sign_seed_bytes(&self) -> usize {
        8
    }

    #[inline]
    fn sign_public_key_bytes(&self) -> usize {
        8
    }

    #[inline]
    fn sign_secret_key_bytes(&self) -> usize {
        8
    }

    #[inline]
    fn sign_bytes(&self) -> usize {
        8
    }

    fn sign_seed_keypair<SeedBuffer: Buffer, PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        &self,
        seed: &SeedBuffer,
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()> {
        public_key.write_lock().write(0, &seed.read_lock())?;
        secret_key.write_lock().write(0, &seed.read_lock())?;
        Ok(())
    }

    fn sign<SignatureBuffer: Buffer, MessageBuffer: Buffer, SecretKeyBuffer: Buffer>(
        &self,
        signature: &mut SignatureBuffer,
        message: &MessageBuffer,
        secret_key: &SecretKeyBuffer,
    ) -> CryptoResult<()> {
        let mut signature = signature.write_lock();
        signature[0] = secret_key.read_lock()[0];
        signature[1] = message.read_lock()[0];
        signature[2] = 0xff;
        signature[3] = 0xff;
        Ok(())
    }

    fn sign_verify<SignatureBuffer: Buffer, MessageBuffer: Buffer, PublicKeyBuffer: Buffer>(
        &self,
        signature: &SignatureBuffer,
        message: &MessageBuffer,
        public_key: &PublicKeyBuffer,
    ) -> CryptoResult<bool> {
        let public_key = public_key.read_lock();
        let message = message.read_lock();
        let signature = signature.read_lock();
        Ok(signature[0] == public_key[0]
            && signature[1] == message[0]
            && signature[2] == 0xff
            && signature[3] == 0xff)
    }
}

lazy_static! {
    static ref FAKE_CRYPTO_SYSTEM: FakeCryptoSystem = FakeCryptoSystem {};
}

impl CryptoSystem for FakeCryptoSystem {
    fn get() -> &'static Self {
        &FAKE_CRYPTO_SYSTEM
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

        FakeCryptoSystem::get().randombytes_buf(&mut buf1).unwrap();

        assert_ne!(
            "InsecureBuffer { b: [0, 0, 0, 0, 0, 0, 0, 0], p: RefCell { value: NoAccess } }",
            &format!("{:?}", buf1)
        );

        let mut buf2 = InsecureBuffer::new(8).unwrap();

        FakeCryptoSystem::get().randombytes_buf(&mut buf2).unwrap();

        assert_ne!(&format!("{:?}", buf1), &format!("{:?}", buf2));
    }

    #[test]
    fn it_should_randomize_vec_u8() {
        let mut buf1 = vec![0; 8];

        assert_eq!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", buf1));

        FakeCryptoSystem::get().randombytes_buf(&mut buf1).unwrap();

        assert_ne!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", buf1));

        let mut buf2 = vec![0; 8];

        FakeCryptoSystem::get().randombytes_buf(&mut buf2).unwrap();

        assert_ne!(&format!("{:?}", buf1), &format!("{:?}", buf2));
    }

    #[test]
    fn it_should_sign_and_verify() {
        let crypto = FakeCryptoSystem::get();

        let data = InsecureBuffer::new(8).unwrap();

        let mut seed = InsecureBuffer::new(crypto.sign_seed_bytes()).unwrap();
        crypto.randombytes_buf(&mut seed).unwrap();

        let mut pub_key = InsecureBuffer::new(crypto.sign_public_key_bytes()).unwrap();
        let mut priv_key = InsecureBuffer::new(crypto.sign_secret_key_bytes()).unwrap();

        crypto
            .sign_seed_keypair(&seed, &mut pub_key, &mut priv_key)
            .unwrap();

        let mut sig = InsecureBuffer::new(crypto.sign_bytes()).unwrap();

        assert!(!crypto.sign_verify(&sig, &data, &pub_key).unwrap());

        crypto.sign(&mut sig, &data, &priv_key).unwrap();

        assert!(crypto.sign_verify(&sig, &data, &pub_key).unwrap());
    }
}
