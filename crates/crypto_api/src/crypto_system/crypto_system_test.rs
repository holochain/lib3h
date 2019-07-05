use crate::{Buffer, InsecureBuffer, CryptoSystem, CryptoResult, CryptoError};

struct FullSuite {
    crypto: Box<CryptoSystem>,
}

impl FullSuite {
    pub fn new(crypto: Box<CryptoSystem>) -> Self {
        FullSuite { crypto }
    }

    pub fn run(&self) {
        self.test_sec_buf();
        self.test_random();
        self.test_sign_keypair_sizes();
        self.test_sign_keypair_generation();
        self.test_sign();
    }

    fn test_sec_buf(&self) {
        let mut b1 = self.crypto.sec_buf_new(8);
        assert_eq!(8, b1.len());
        assert_eq!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", &b1.read_lock()));
        b1.write(0, &[42, 88, 132, 56, 12, 254, 212, 88]).unwrap();
        assert_eq!("[42, 88, 132, 56, 12, 254, 212, 88]", &format!("{:?}", &b1.read_lock()));
        let b2 = b1.box_clone();
        b1.zero();
        assert_eq!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", &b1.read_lock()));
        assert_eq!("[42, 88, 132, 56, 12, 254, 212, 88]", &format!("{:?}", &b2.read_lock()));
    }

    fn test_random(&self) {
        let mut a: Box<dyn Buffer> = Box::new(vec![0; 8]);
        let mut b: Box<dyn Buffer> = Box::new(vec![0; 8]);
        assert_eq!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", a));
        self.crypto.randombytes_buf(&mut a).unwrap();
        self.crypto.randombytes_buf(&mut b).unwrap();
        assert_ne!("[0, 0, 0, 0, 0, 0, 0, 0]", &format!("{:?}", a));
        assert_ne!(&format!("{:?}", a), &format!("{:?}", b));
    }

    fn test_sign_keypair_sizes(&self) {
        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes() + 1]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        assert_eq!(CryptoError::BadSeedSize, self.crypto.sign_seed_keypair(&seed, &mut pk, &mut sk).unwrap_err());

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes() + 1]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        assert_eq!(CryptoError::BadPublicKeySize, self.crypto.sign_seed_keypair(&seed, &mut pk, &mut sk).unwrap_err());
        assert_eq!(CryptoError::BadPublicKeySize, self.crypto.sign_keypair(&mut pk, &mut sk).unwrap_err());

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes() + 1]);
        assert_eq!(CryptoError::BadSecretKeySize, self.crypto.sign_seed_keypair(&seed, &mut pk, &mut sk).unwrap_err());
        assert_eq!(CryptoError::BadSecretKeySize, self.crypto.sign_keypair(&mut pk, &mut sk).unwrap_err());
    }

    fn test_sign_keypair_generation(&self) {
        let mut seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        let mut pk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);

        self.crypto.sign_seed_keypair(&seed, &mut pk1, &mut sk1).unwrap();
        self.crypto.sign_seed_keypair(&seed, &mut pk2, &mut sk2).unwrap();
        assert_eq!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_eq!(&format!("{:?}", sk1), &format!("{:?}", sk2));

        self.crypto.randombytes_buf(&mut seed).unwrap();
        self.crypto.sign_seed_keypair(&seed, &mut pk2, &mut sk2).unwrap();
        assert_ne!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_ne!(&format!("{:?}", sk1), &format!("{:?}", sk2));

        self.crypto.sign_keypair(&mut pk1, &mut sk1).unwrap();
        assert_ne!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_ne!(&format!("{:?}", sk1), &format!("{:?}", sk2));
    }

    fn test_sign(&self) {
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        let mut msg: Box<dyn Buffer> = Box::new(vec![0; 64]);
        self.crypto.randombytes_buf(&mut msg).unwrap();

        self.crypto.sign_keypair(&mut pk, &mut sk).unwrap();

        let mut sig: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_bytes()]);
        self.crypto.sign(&mut sig, &msg, &sk).unwrap();
        assert!(self.crypto.sign_verify(&sig, &msg, &pk).unwrap());

        self.crypto.randombytes_buf(&mut sig).unwrap();
        assert!(!self.crypto.sign_verify(&sig, &msg, &pk).unwrap());
    }
}

pub fn full_suite(crypto: Box<CryptoSystem>) {
    FullSuite::new(crypto).run();
}

#[test]
fn it_should_pass_fake_full_suite() {
    full_suite(Box::new(FakeCryptoSystem));
}

struct FakeCryptoSystem;

impl CryptoSystem for FakeCryptoSystem {
    fn sec_buf_new(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(InsecureBuffer::new(size))
    }

    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()> {
        let mut buffer = buffer.write_lock();

        for i in 0..buffer.len() {
            buffer[i] = rand::random();
        }

        Ok(())
    }

    fn sign_seed_bytes(&self) -> usize { 8 }
    fn sign_public_key_bytes(&self) -> usize { 32 }
    fn sign_secret_key_bytes(&self) -> usize { 8 }
    fn sign_bytes(&self) -> usize { 16 }

    fn sign_seed_keypair(&self, seed: &Box<dyn Buffer>, public_key: &mut Box<dyn Buffer>, secret_key: &mut Box<dyn Buffer>) -> CryptoResult<()> {
        if seed.len() != self.sign_seed_bytes() {
            return Err(CryptoError::BadSeedSize);
        }

        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        secret_key.write(0, &seed.read_lock())?;

        public_key.zero();
        public_key.write(0, &seed.read_lock())?;

        Ok(())
    }

    fn sign_keypair(&self, public_key: &mut Box<dyn Buffer>, secret_key: &mut Box<dyn Buffer>) -> CryptoResult<()> {
        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut seed: Box<dyn Buffer> = Box::new(vec![0; self.sign_seed_bytes()]);
        self.randombytes_buf(&mut seed)?;
        self.sign_seed_keypair(&seed, public_key, secret_key)?;

        Ok(())
    }

    fn sign(&self, signature: &mut Box<dyn Buffer>, message: &Box<dyn Buffer>, secret_key: &Box<dyn Buffer>) -> CryptoResult<()> {
        if signature.len() != self.sign_bytes() {
            return Err(CryptoError::BadSignatureSize);
        }

        if secret_key.len() != self.sign_secret_key_bytes() {
            return Err(CryptoError::BadSecretKeySize);
        }

        signature.write(0, &secret_key.read_lock())?;
        let mlen = if message.len() > 8 { 8 } else { message.len() };
        signature.write(8, &message.read_lock()[0..mlen])?;

        Ok(())
    }

    fn sign_verify(&self, signature: &Box<dyn Buffer>, message: &Box<dyn Buffer>, public_key: &Box<dyn Buffer>) -> CryptoResult<bool> {
        if signature.len() != self.sign_bytes() {
            return Err(CryptoError::BadSignatureSize);
        }

        if public_key.len() != self.sign_public_key_bytes() {
            return Err(CryptoError::BadPublicKeySize);
        }

        let signature = signature.read_lock();
        let mlen = if message.len() > 8 { 8 } else { message.len() };

        Ok(
            &signature[0..8] == &public_key.read_lock()[0..8] &&
            &signature[8..mlen + 8] == &message.read_lock()[0..mlen]
        )
    }
}
