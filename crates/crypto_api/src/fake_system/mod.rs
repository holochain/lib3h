use crate::{Buffer, CryptoRandom, CryptoResult, CryptoSignature, CryptoSystem};

/// WARNING THIS IS NOT SECURE!!
/// This is a fake crypto system to give hints for implementing real systems.
/// The functions here mimic a real crypto system, but are doing trivial things.
/// Do not use this for any real systems.
/// Even the random functions are fake, and produce poor results.
pub struct FakeCryptoSystem {}

const FAKE_SEQ: [u32; 4] = [0x30698bae, 0x47984c92, 0x901d24fb, 0x91fba506];

impl CryptoRandom for FakeCryptoSystem {
    // rust doesn't supply any random functions in std
    // and we don't want to import another crate just for this
    // mimic randomness using subsec_nanos() BUT DON'T RELY ON THIS!
    fn randombytes_buf<OutputBuffer: Buffer>(buffer: &mut OutputBuffer) -> CryptoResult<()> {
        let mut buffer = buffer.write_lock();

        let mut idx = 0;
        let mut seed = 0;

        for i in 0..buffer.len() {
            seed = seed
                ^ FAKE_SEQ[idx]
                ^ std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
            idx += 1;
            if idx >= FAKE_SEQ.len() {
                idx = 0;
            }
            buffer[i] = (seed % 256) as u8;
        }

        Ok(())
    }
}

impl CryptoSignature for FakeCryptoSystem {
    const SIGN_SEED_BYTES: usize = 8;
    const SIGN_PUBLIC_KEY_BYTES: usize = 8;
    const SIGN_SECRET_KEY_BYTES: usize = 8;
    const SIGN_BYTES: usize = 8;

    fn sign_seed_keypair<SeedBuffer: Buffer, PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        seed: &SeedBuffer,
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()> {
        public_key.write_lock().write(0, &seed.read_lock())?;
        secret_key.write_lock().write(0, &seed.read_lock())?;
        Ok(())
    }

    fn sign<SignatureBuffer: Buffer, MessageBuffer: Buffer, SecretKeyBuffer: Buffer>(
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

impl CryptoSystem for FakeCryptoSystem {}

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

        FakeCryptoSystem::randombytes_buf(&mut buf1).unwrap();

        assert_ne!(
            "InsecureBuffer { b: [0, 0, 0, 0, 0, 0, 0, 0], p: RefCell { value: NoAccess } }",
            &format!("{:?}", buf1)
        );

        let mut buf2 = InsecureBuffer::new(8).unwrap();

        FakeCryptoSystem::randombytes_buf(&mut buf2).unwrap();

        assert_ne!(&format!("{:?}", buf1), &format!("{:?}", buf2));
    }

    #[test]
    fn it_should_randomize_vec_u8() {
        let mut buf1 = vec![0; 8];

        assert_eq!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", buf1));

        FakeCryptoSystem::randombytes_buf(&mut buf1).unwrap();

        assert_ne!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", buf1));

        let mut buf2 = vec![0; 8];

        FakeCryptoSystem::randombytes_buf(&mut buf2).unwrap();

        assert_ne!(&format!("{:?}", buf1), &format!("{:?}", buf2));
    }

    #[test]
    fn it_should_sign_and_verify() {
        let data = InsecureBuffer::new(8).unwrap();

        let mut seed = InsecureBuffer::new(FakeCryptoSystem::SIGN_SEED_BYTES).unwrap();
        FakeCryptoSystem::randombytes_buf(&mut seed).unwrap();

        let mut pub_key = InsecureBuffer::new(FakeCryptoSystem::SIGN_PUBLIC_KEY_BYTES).unwrap();
        let mut priv_key = InsecureBuffer::new(FakeCryptoSystem::SIGN_SECRET_KEY_BYTES).unwrap();

        FakeCryptoSystem::sign_seed_keypair(&seed, &mut pub_key, &mut priv_key).unwrap();

        let mut sig = InsecureBuffer::new(FakeCryptoSystem::SIGN_BYTES).unwrap();

        assert!(!FakeCryptoSystem::sign_verify(&sig, &data, &pub_key).unwrap());

        FakeCryptoSystem::sign(&mut sig, &data, &priv_key).unwrap();

        assert!(FakeCryptoSystem::sign_verify(&sig, &data, &pub_key).unwrap());
    }
}
