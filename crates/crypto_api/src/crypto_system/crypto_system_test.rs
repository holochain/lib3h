//! Expose a test suite that can exercise CryptoSystem implementations.
//! You'll probably also need to write unit tests specific to your impl.

use crate::{Buffer, CryptoError, CryptoSystem};

struct FullSuite {
    crypto: Box<dyn CryptoSystem>,
}

impl FullSuite {
    pub fn new(crypto: Box<dyn CryptoSystem>) -> Self {
        FullSuite { crypto }
    }

    pub fn run(&self) {
        self.test_sec_buf();
        self.test_random();
        self.test_hash();
        self.test_generic_hash();
        self.test_pwhash();
        self.test_kdf();
        self.test_sign_keypair_sizes();
        self.test_sign_keypair_generation();
        self.test_sign();
        self.test_kx_keypair_sizes();
        self.test_kx_keypair_generation();
        self.test_kx();
        self.test_aead();
    }

    #[allow(clippy::cognitive_complexity)]
    fn test_sec_buf(&self) {
        let mut b1 = self.crypto.buf_new_secure(8);
        assert_eq!(8, b1.len());
        assert_eq!(
            "[0, 0, 0, 0, 0, 0, 0, 0]",
            &format!("{:?}", &b1.read_lock())
        );
        b1.write(0, &[42, 88, 132, 56, 12, 254, 212, 88]).unwrap();
        assert_eq!(
            "[42, 88, 132, 56, 12, 254, 212, 88]",
            &format!("{:?}", &b1.read_lock())
        );
        let b2 = b1.box_clone();
        b1.zero();
        assert_eq!(
            "[0, 0, 0, 0, 0, 0, 0, 0]",
            &format!("{:?}", &b1.read_lock())
        );
        assert_eq!(
            "[42, 88, 132, 56, 12, 254, 212, 88]",
            &format!("{:?}", &b2.read_lock())
        );

        // test compare
        let mut z1 = self.crypto.buf_new_secure(0);
        let mut z2 = self.crypto.buf_new_secure(0);

        let mut a = self.crypto.buf_new_secure(1);
        {
            let mut a = a.write_lock();
            a[0] = 50;
        }
        let mut b = self.crypto.buf_new_secure(1);
        {
            let mut b = b.write_lock();
            b[0] = 45;
        }
        let mut c = self.crypto.buf_new_secure(1);
        {
            let mut c = c.write_lock();
            c[0] = 45;
        }
        let mut d0 = self.crypto.buf_new_secure(2);
        {
            let mut d0 = d0.write_lock();
            d0[0] = 45;
            d0[1] = 0;
        }
        let mut d1 = self.crypto.buf_new_secure(2);
        {
            let mut d1 = d1.write_lock();
            d1[0] = 45;
            d1[1] = 1;
        }
        let mut d045 = self.crypto.buf_new_secure(2);
        {
            let mut d045 = d045.write_lock();
            d045[0] = 0;
            d045[1] = 45;
        }
        let mut e0 = self.crypto.buf_new_secure(2);
        {
            let mut e0 = e0.write_lock();
            e0[0] = 0;
            e0[1] = 2;
        }
        let mut e2 = self.crypto.buf_new_secure(2);
        {
            let mut e2 = e2.write_lock();
            e2[0] = 2;
            e2[1] = 0;
        }
        let mut f = self.crypto.buf_new_secure(3);
        {
            let mut f = f.write_lock();
            f[0] = 1;
            f[1] = 1;
            f[2] = 1;
        }

        // compare length 0 sized buffers
        assert_eq!(z1.compare(&mut z2), 0);
        assert_eq!(z2.compare(&mut z1), 0);

        // compare length 1 sized buffers
        assert_eq!(b.compare(&mut c), 0); // [45] == [45]
        assert_eq!(a.compare(&mut b), 1); // [50] > [45]
        assert_eq!(b.compare(&mut a), -1); // [45] < [50]

        // compare length 2 sized buffers
        assert_eq!(d1.compare(&mut e0), -1); // [45, 1] < [0, 2]
        assert_eq!(e0.compare(&mut d1), 1); // [0, 2] > [45, 1]
        assert_eq!(d0.compare(&mut e0), -1); // [45, 0] < [0, 2]
        assert_eq!(e0.compare(&mut d0), 1); // [0, 2] > [45, 0]

        assert_eq!(d1.compare(&mut e2), 1); // [45, 1] > [2, 0]
        assert_eq!(e2.compare(&mut d1), -1); // [2, 0] < [45, 1]
        assert_eq!(d0.compare(&mut e2), 1); // [45, 0] > [2, 0]
        assert_eq!(e2.compare(&mut d0), -1); // [2, 0] < [45, 0]

        assert_eq!(d1.compare(&mut d0), 1); // [45, 1] > [45, 0]
        assert_eq!(d0.compare(&mut d1), -1); // [45, 0] < [45, 1]

        assert_eq!(d045.compare(&mut d0), 1); // [0, 45] > [45, 0]
        assert_eq!(d0.compare(&mut d045), -1); // [45, 0] < [0, 45]

        // compare different sized buffers
        assert_eq!(c.compare(&mut d1), 0); // [45] == [45, 1]
        assert_eq!(c.compare(&mut d0), 0); // [45] == [45, 0]
        assert_eq!(c.compare(&mut d045), 1); // [45] > [0, 45]
        assert_eq!(d0.compare(&mut c), 0); // [45, 0] == [45]
        assert_eq!(d1.compare(&mut c), 1); // [45, 1] > [45]
        assert_eq!(d045.compare(&mut c), 1); // [0, 45] > [45]

        //
        assert_eq!(a.compare(&mut d1), 1); // [50] > [45, 1]
        assert_eq!(d1.compare(&mut a), 1); // [45, 1] > [50]

        assert_eq!(e0.compare(&mut a), 1); // [0, 2] > [50]
        assert_eq!(a.compare(&mut e0), 1); // [50] > [0, 2]

        // compare different sized buffers
        assert_eq!(f.compare(&mut e0), 1); // [1, 1, 1] > [1, 2]
        assert_eq!(e0.compare(&mut f), 1); // [0, 2] > [1, 1, 1]
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

    fn test_hash(&self) {
        let data: Box<dyn Buffer> = Box::new(vec![42, 1, 38, 2, 155, 212, 3, 11]);

        let mut hash256: Box<dyn Buffer> = Box::new(vec![0; self.crypto.hash_sha256_bytes()]);
        self.crypto.hash_sha256(&mut hash256, &data).unwrap();
        assert_eq!("[69, 32, 143, 143, 29, 27, 233, 62, 97, 209, 120, 159, 137, 193, 1, 213, 107, 128, 33, 170, 165, 131, 217, 170, 66, 192, 214, 190, 20, 179, 219, 177]", &format!("{:?}", hash256));

        let mut hash512: Box<dyn Buffer> = Box::new(vec![0; self.crypto.hash_sha512_bytes()]);
        self.crypto.hash_sha512(&mut hash512, &data).unwrap();
        assert_eq!("[105, 206, 48, 255, 80, 134, 192, 184, 108, 217, 124, 49, 193, 43, 2, 219, 148, 27, 91, 154, 89, 69, 229, 78, 13, 74, 51, 57, 52, 201, 186, 25, 109, 206, 155, 242, 249, 8, 179, 34, 106, 170, 160, 158, 11, 89, 85, 25, 22, 70, 70, 150, 84, 221, 184, 130, 245, 196, 101, 192, 160, 225, 160, 253]", &format!("{:?}", hash512));
    }

    fn test_generic_hash(&self) {
        let data: Box<dyn Buffer> = Box::new(vec![42, 1, 38, 2, 155, 212, 3, 11]);

        let mut hash1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.generic_hash_min_bytes()]);
        let mut hash2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.generic_hash_min_bytes()]);

        self.crypto.generic_hash(&mut hash1, &data, None).unwrap();
        assert_ne!(*hash1.read_lock(), *hash2.read_lock());

        self.crypto.generic_hash(&mut hash2, &data, None).unwrap();
        assert_eq!(*hash1.read_lock(), *hash2.read_lock());
    }

    fn test_pwhash(&self) {
        let mut pw: Box<dyn Buffer> = Box::new(vec![0; 16]);
        self.crypto.randombytes_buf(&mut pw).unwrap();
        let mut salt: Box<dyn Buffer> = Box::new(vec![0; self.crypto.pwhash_salt_bytes()]);
        self.crypto.randombytes_buf(&mut salt).unwrap();
        let mut hash1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.pwhash_bytes()]);
        self.crypto.pwhash(&mut hash1, &pw, &salt).unwrap();
        let mut hash2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.pwhash_bytes()]);
        self.crypto.pwhash(&mut hash2, &pw, &salt).unwrap();
        assert_eq!(&format!("{:?}", hash1), &format!("{:?}", hash2));
    }

    fn test_kdf(&self) {
        let ctx1: Box<dyn Buffer> = Box::new(vec![1; self.crypto.kdf_context_bytes()]);
        let ctx2: Box<dyn Buffer> = Box::new(vec![2; self.crypto.kdf_context_bytes()]);

        let root: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_key_bytes()]);
        let mut a_1_1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);
        let mut a_2_1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);
        let mut a_1_2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);
        let mut b_1_1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);
        let mut b_2_1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);
        let mut b_1_2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kdf_min_bytes()]);

        self.crypto.kdf(&mut a_1_1, 1, &ctx1, &root).unwrap();
        self.crypto.kdf(&mut a_2_1, 2, &ctx1, &root).unwrap();
        self.crypto.kdf(&mut a_1_2, 1, &ctx2, &root).unwrap();

        self.crypto.kdf(&mut b_1_1, 1, &ctx1, &root).unwrap();
        self.crypto.kdf(&mut b_2_1, 2, &ctx1, &root).unwrap();
        self.crypto.kdf(&mut b_1_2, 1, &ctx2, &root).unwrap();

        assert_eq!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", b_1_1),
            "a_1_1 == b_1_1"
        );
        assert_eq!(
            &format!("{:?}", a_2_1),
            &format!("{:?}", b_2_1),
            "a_2_1 == b_2_1"
        );
        assert_eq!(
            &format!("{:?}", a_1_2),
            &format!("{:?}", b_1_2),
            "a_1_2 == b_1_2"
        );

        assert_ne!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", a_2_1),
            "a_1_1 != a_2_1"
        );
        assert_ne!(
            &format!("{:?}", a_1_1),
            &format!("{:?}", a_1_2),
            "a_1_1 != a_1_2"
        );
        assert_ne!(
            &format!("{:?}", a_2_1),
            &format!("{:?}", a_1_2),
            "a_2_1 != a_1_2"
        );
    }

    fn test_sign_keypair_sizes(&self) {
        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes() + 1]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        assert_eq!(
            CryptoError::BadSeedSize,
            self.crypto
                .sign_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes() + 1]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        assert_eq!(
            CryptoError::BadPublicKeySize,
            self.crypto
                .sign_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );
        assert_eq!(
            CryptoError::BadPublicKeySize,
            self.crypto.sign_keypair(&mut pk, &mut sk).unwrap_err()
        );

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes() + 1]);
        assert_eq!(
            CryptoError::BadSecretKeySize,
            self.crypto
                .sign_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );
        assert_eq!(
            CryptoError::BadSecretKeySize,
            self.crypto.sign_keypair(&mut pk, &mut sk).unwrap_err()
        );
    }

    fn test_sign_keypair_generation(&self) {
        let mut seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_seed_bytes()]);
        let mut pk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);
        let mut pk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_public_key_bytes()]);
        let mut sk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.sign_secret_key_bytes()]);

        self.crypto
            .sign_seed_keypair(&seed, &mut pk1, &mut sk1)
            .unwrap();
        self.crypto
            .sign_seed_keypair(&seed, &mut pk2, &mut sk2)
            .unwrap();
        assert_eq!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_eq!(&format!("{:?}", sk1), &format!("{:?}", sk2));

        self.crypto.randombytes_buf(&mut seed).unwrap();
        self.crypto
            .sign_seed_keypair(&seed, &mut pk2, &mut sk2)
            .unwrap();
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

    fn test_kx_keypair_sizes(&self) {
        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_seed_bytes() + 1]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);
        assert_eq!(
            CryptoError::BadSeedSize,
            self.crypto
                .kx_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes() + 1]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);
        assert_eq!(
            CryptoError::BadPublicKeySize,
            self.crypto
                .kx_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );
        assert_eq!(
            CryptoError::BadPublicKeySize,
            self.crypto.kx_keypair(&mut pk, &mut sk).unwrap_err()
        );

        let seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_seed_bytes()]);
        let mut pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes() + 1]);
        assert_eq!(
            CryptoError::BadSecretKeySize,
            self.crypto
                .kx_seed_keypair(&seed, &mut pk, &mut sk)
                .unwrap_err()
        );
        assert_eq!(
            CryptoError::BadSecretKeySize,
            self.crypto.kx_keypair(&mut pk, &mut sk).unwrap_err()
        );
    }

    fn test_kx_keypair_generation(&self) {
        let mut seed: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_seed_bytes()]);
        let mut pk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut sk1: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);
        let mut pk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut sk2: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);

        self.crypto
            .kx_seed_keypair(&seed, &mut pk1, &mut sk1)
            .unwrap();
        self.crypto
            .kx_seed_keypair(&seed, &mut pk2, &mut sk2)
            .unwrap();
        assert_eq!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_eq!(&format!("{:?}", sk1), &format!("{:?}", sk2));

        self.crypto.randombytes_buf(&mut seed).unwrap();
        self.crypto
            .kx_seed_keypair(&seed, &mut pk2, &mut sk2)
            .unwrap();
        assert_ne!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_ne!(&format!("{:?}", sk1), &format!("{:?}", sk2));

        self.crypto.kx_keypair(&mut pk1, &mut sk1).unwrap();
        assert_ne!(&format!("{:?}", pk1), &format!("{:?}", pk2));
        assert_ne!(&format!("{:?}", sk1), &format!("{:?}", sk2));
    }

    fn test_kx(&self) {
        let mut c_pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut c_sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);
        let mut s_pk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_public_key_bytes()]);
        let mut s_sk: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_secret_key_bytes()]);

        self.crypto.kx_keypair(&mut c_pk, &mut c_sk).unwrap();
        self.crypto.kx_keypair(&mut s_pk, &mut s_sk).unwrap();

        let mut c_rx: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_session_key_bytes()]);
        let mut c_tx: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_session_key_bytes()]);
        let mut s_rx: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_session_key_bytes()]);
        let mut s_tx: Box<dyn Buffer> = Box::new(vec![0; self.crypto.kx_session_key_bytes()]);

        //self.crypto
        //    .kx_client_session_keys(&mut c_rx, &mut c_tx, &c_pk, &c_sk, &s_pk)
        //    .unwrap();
        kx_client_session_keys!(self.crypto =>
            client_rx: &mut c_rx,
            client_tx: &mut c_tx,
            client_pk: &c_pk,
            client_sk: &c_sk,
            server_pk: &s_pk,
        )
        .unwrap();
        //self.crypto
        //    .kx_server_session_keys(&mut s_rx, &mut s_tx, &s_pk, &s_sk, &c_pk)
        //    .unwrap();
        kx_server_session_keys!(self.crypto =>
            server_rx: &mut s_rx,
            server_tx: &mut s_tx,
            server_pk: &s_pk,
            server_sk: &s_sk,
            client_pk: &c_pk,
        )
        .unwrap();

        assert_ne!(&format!("{:?}", c_rx), &format!("{:?}", s_rx));
        assert_ne!(&format!("{:?}", c_tx), &format!("{:?}", s_tx));

        assert_eq!(&format!("{:?}", c_rx), &format!("{:?}", s_tx));
        assert_eq!(&format!("{:?}", c_tx), &format!("{:?}", s_rx));
    }

    fn test_aead(&self) {
        let mut secret: Box<dyn Buffer> = Box::new(vec![0; self.crypto.aead_secret_bytes()]);
        self.crypto.randombytes_buf(&mut secret).unwrap();
        let mut nonce: Box<dyn Buffer> = Box::new(vec![0; self.crypto.aead_nonce_bytes()]);
        self.crypto.randombytes_buf(&mut nonce).unwrap();
        let mut message: Box<dyn Buffer> = Box::new(vec![0; 16]);
        self.crypto.randombytes_buf(&mut message).unwrap();
        let mut adata: Box<dyn Buffer> = Box::new(vec![0; 16]);
        self.crypto.randombytes_buf(&mut adata).unwrap();

        let mut cipher: Box<dyn Buffer> = Box::new(vec![0; 16 + self.crypto.aead_auth_bytes()]);

        //self.crypto
        //    .aead_encrypt(&mut cipher, &message, Some(&adata), &nonce, &secret)
        //    .unwrap();
        aead_encrypt!(self.crypto =>
            cipher: &mut cipher,
            message: &message,
            adata: Some(&adata),
            nonce: &nonce,
            secret: &secret,
        )
        .unwrap();

        let mut msg_out: Box<dyn Buffer> =
            Box::new(vec![0; cipher.len() - self.crypto.aead_auth_bytes()]);

        //self.crypto
        //    .aead_decrypt(&mut msg_out, &cipher, Some(&adata), &nonce, &secret)
        //    .unwrap();
        aead_decrypt!(self.crypto =>
            message: &mut msg_out,
            cipher: &cipher,
            adata: Some(&adata),
            nonce: &nonce,
            secret: &secret,
        )
        .unwrap();

        assert_eq!(&format!("{:?}", message), &format!("{:?}", msg_out));
    }
}

/// run a full suite of common CryptoSystem verification functions
pub fn full_suite(crypto: Box<dyn CryptoSystem>) {
    FullSuite::new(crypto).run();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{CryptoResult, ProtectState};
    use rand::{Rng, SeedableRng};
    use sha2::Digest;
    use std::ops::{Deref, DerefMut};

    #[test]
    fn fake_should_pass_crypto_system_full_suite() {
        let seed: [u8; 32] = [
            143, 106, 67, 237, 112, 106, 175, 150, 195, 103, 30, 19, 109, 13, 220, 160, 31, 212,
            59, 142, 251, 44, 63, 50, 123, 52, 6, 104, 201, 223, 19, 140,
        ];
        full_suite(Box::new(FakeCryptoSystem {
            seed: seed.clone(),
            rng: std::sync::RwLock::new(rand::rngs::StdRng::from_seed(seed)),
        }));
    }

    #[derive(Debug, Clone)]
    pub struct InsecureBuffer {
        b: Box<[u8]>,
        p: std::cell::RefCell<ProtectState>,
    }

    impl InsecureBuffer {
        pub fn new(size: usize) -> Self {
            InsecureBuffer {
                b: vec![0; size].into_boxed_slice(),
                p: std::cell::RefCell::new(ProtectState::NoAccess),
            }
        }
    }

    impl Deref for InsecureBuffer {
        type Target = [u8];

        fn deref(&self) -> &Self::Target {
            if *self.p.borrow() == ProtectState::NoAccess {
                panic!("Deref, but state is NoAccess");
            }
            &self.b
        }
    }

    impl DerefMut for InsecureBuffer {
        fn deref_mut(&mut self) -> &mut Self::Target {
            if *self.p.borrow() != ProtectState::ReadWrite {
                panic!("DerefMut, but state is not ReadWrite");
            }
            &mut self.b
        }
    }

    impl Buffer for InsecureBuffer {
        fn box_clone(&self) -> Box<dyn Buffer> {
            Box::new(self.clone())
        }
        fn as_buffer(&self) -> &dyn Buffer {
            &*self
        }
        fn as_buffer_mut(&mut self) -> &mut dyn Buffer {
            &mut *self
        }
        fn len(&self) -> usize {
            self.b.len()
        }
        fn is_empty(&self) -> bool {
            self.b.is_empty()
        }
        fn set_no_access(&self) {
            if *self.p.borrow() == ProtectState::NoAccess {
                panic!("already no access... bad logic");
            }
            *self.p.borrow_mut() = ProtectState::NoAccess;
        }
        fn set_readable(&self) {
            if *self.p.borrow() != ProtectState::NoAccess {
                panic!("not no access... bad logic");
            }
            *self.p.borrow_mut() = ProtectState::ReadOnly;
        }
        fn set_writable(&self) {
            if *self.p.borrow() != ProtectState::NoAccess {
                panic!("not no access... bad logic");
            }
            *self.p.borrow_mut() = ProtectState::ReadWrite;
        }
    }

    struct FakeCryptoSystem {
        seed: [u8; 32],
        rng: std::sync::RwLock<rand::rngs::StdRng>,
    }

    impl CryptoSystem for FakeCryptoSystem {
        fn box_clone(&self) -> Box<dyn CryptoSystem> {
            Box::new(FakeCryptoSystem {
                seed: self.seed.clone(),
                rng: std::sync::RwLock::new(rand::rngs::StdRng::from_seed(self.seed.clone())),
            })
        }

        fn as_crypto_system(&self) -> &dyn CryptoSystem {
            &*self
        }

        fn buf_new_secure(&self, size: usize) -> Box<dyn Buffer> {
            Box::new(InsecureBuffer::new(size))
        }

        fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()> {
            let mut buffer = buffer.write_lock();

            for i in 0..buffer.len() {
                buffer[i] = self.rng.write().unwrap().gen();
            }

            Ok(())
        }

        fn hash_sha256_bytes(&self) -> usize {
            32
        }
        fn hash_sha512_bytes(&self) -> usize {
            64
        }

        fn hash_sha256(
            &self,
            hash: &mut Box<dyn Buffer>,
            data: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if hash.len() != self.hash_sha256_bytes() {
                return Err(CryptoError::BadHashSize);
            }

            let mut hasher = sha2::Sha256::new();
            hasher.input(data.read_lock().deref());
            hash.write(0, &hasher.result())?;
            Ok(())
        }

        fn hash_sha512(
            &self,
            hash: &mut Box<dyn Buffer>,
            data: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if hash.len() != self.hash_sha512_bytes() {
                return Err(CryptoError::BadHashSize);
            }

            let mut hasher = sha2::Sha512::new();
            hasher.input(data.read_lock().deref());
            hash.write(0, &hasher.result())?;
            Ok(())
        }

        fn generic_hash_min_bytes(&self) -> usize {
            8
        }

        fn generic_hash_max_bytes(&self) -> usize {
            8
        }

        fn generic_hash_key_min_bytes(&self) -> usize {
            8
        }

        fn generic_hash_key_max_bytes(&self) -> usize {
            8
        }

        fn generic_hash(
            &self,
            hash: &mut Box<dyn Buffer>,
            data: &Box<dyn Buffer>,
            key: Option<&Box<dyn Buffer>>,
        ) -> CryptoResult<()> {
            if hash.len() < self.generic_hash_min_bytes()
                || hash.len() > self.generic_hash_max_bytes()
            {
                return Err(CryptoError::BadHashSize);
            }

            if key.is_some()
                && (key.unwrap().len() < self.generic_hash_key_min_bytes()
                    || key.unwrap().len() > self.generic_hash_key_max_bytes())
            {
                return Err(CryptoError::BadKeySize);
            }

            let mut hasher = sha2::Sha512::new();
            hasher.input(data.read_lock().deref());
            hash.write(0, &hasher.result()[..hash.len()])?;
            Ok(())
        }

        fn pwhash_salt_bytes(&self) -> usize {
            8
        }
        fn pwhash_bytes(&self) -> usize {
            16
        }

        fn pwhash(
            &self,
            hash: &mut Box<dyn Buffer>,
            password: &Box<dyn Buffer>,
            salt: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if hash.len() != self.pwhash_bytes() {
                return Err(CryptoError::BadHashSize);
            }

            if salt.len() != self.pwhash_salt_bytes() {
                return Err(CryptoError::BadSaltSize);
            }

            hash.write(0, &salt.read_lock())?;
            let plen = if password.len() > 8 {
                8
            } else {
                password.len()
            };
            hash.write(8, &password.read_lock()[0..plen])?;

            Ok(())
        }

        fn kdf_key_bytes(&self) -> usize {
            8
        }

        fn kdf_context_bytes(&self) -> usize {
            1
        }

        fn kdf_min_bytes(&self) -> usize {
            8
        }

        fn kdf_max_bytes(&self) -> usize {
            8
        }

        fn kdf(
            &self,
            out_buffer: &mut Box<dyn Buffer>,
            index: u64,
            context: &Box<dyn Buffer>,
            parent: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if out_buffer.len() < self.kdf_min_bytes() || out_buffer.len() > self.kdf_max_bytes() {
                return Err(CryptoError::BadOutBufferSize);
            }

            if parent.len() != self.kdf_key_bytes() {
                return Err(CryptoError::BadParentSize);
            }

            if context.len() != self.kdf_context_bytes() {
                return Err(CryptoError::BadContextSize);
            }

            out_buffer.write(0, parent)?;
            let mut out_buffer = out_buffer.write_lock();
            out_buffer[4] = (index % 256) as u8;
            out_buffer[5] = context.read_lock()[0];

            Ok(())
        }

        fn sign_seed_bytes(&self) -> usize {
            8
        }
        fn sign_public_key_bytes(&self) -> usize {
            32
        }
        fn sign_secret_key_bytes(&self) -> usize {
            8
        }
        fn sign_bytes(&self) -> usize {
            16
        }

        fn sign_seed_keypair(
            &self,
            seed: &Box<dyn Buffer>,
            public_key: &mut Box<dyn Buffer>,
            secret_key: &mut Box<dyn Buffer>,
        ) -> CryptoResult<()> {
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

        fn sign_keypair(
            &self,
            public_key: &mut Box<dyn Buffer>,
            secret_key: &mut Box<dyn Buffer>,
        ) -> CryptoResult<()> {
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

        fn sign(
            &self,
            signature: &mut Box<dyn Buffer>,
            message: &Box<dyn Buffer>,
            secret_key: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
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

        fn sign_verify(
            &self,
            signature: &Box<dyn Buffer>,
            message: &Box<dyn Buffer>,
            public_key: &Box<dyn Buffer>,
        ) -> CryptoResult<bool> {
            if signature.len() != self.sign_bytes() {
                return Err(CryptoError::BadSignatureSize);
            }

            if public_key.len() != self.sign_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            let signature = signature.read_lock();
            let mlen = if message.len() > 8 { 8 } else { message.len() };

            Ok(&signature[0..8] == &public_key.read_lock()[0..8]
                && &signature[8..mlen + 8] == &message.read_lock()[0..mlen])
        }

        fn kx_seed_bytes(&self) -> usize {
            8
        }
        fn kx_public_key_bytes(&self) -> usize {
            32
        }
        fn kx_secret_key_bytes(&self) -> usize {
            8
        }
        fn kx_session_key_bytes(&self) -> usize {
            8
        }

        fn kx_seed_keypair(
            &self,
            seed: &Box<dyn Buffer>,
            public_key: &mut Box<dyn Buffer>,
            secret_key: &mut Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if seed.len() != self.kx_seed_bytes() {
                return Err(CryptoError::BadSeedSize);
            }

            if public_key.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            if secret_key.len() != self.kx_secret_key_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            secret_key.write(0, &seed.read_lock())?;

            public_key.zero();
            public_key.write(0, &seed.read_lock())?;

            Ok(())
        }

        fn kx_keypair(
            &self,
            public_key: &mut Box<dyn Buffer>,
            secret_key: &mut Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if public_key.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            if secret_key.len() != self.kx_secret_key_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            let mut seed: Box<dyn Buffer> = Box::new(vec![0; self.sign_seed_bytes()]);
            self.randombytes_buf(&mut seed)?;
            self.kx_seed_keypair(&seed, public_key, secret_key)?;

            Ok(())
        }

        fn kx_client_session_keys(
            &self,
            client_rx: &mut Box<dyn Buffer>,
            client_tx: &mut Box<dyn Buffer>,
            client_pk: &Box<dyn Buffer>,
            client_sk: &Box<dyn Buffer>,
            server_pk: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if client_rx.len() != self.kx_session_key_bytes() {
                return Err(CryptoError::BadRxSessionKeySize);
            }

            if client_tx.len() != self.kx_session_key_bytes() {
                return Err(CryptoError::BadTxSessionKeySize);
            }

            if client_pk.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            if client_sk.len() != self.kx_secret_key_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            if server_pk.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            client_rx.write(0, &client_pk.read_lock()[..4])?;
            client_rx.write(4, &server_pk.read_lock()[..4])?;
            client_tx.write(0, &server_pk.read_lock()[..4])?;
            client_tx.write(4, &client_pk.read_lock()[..4])?;

            Ok(())
        }

        fn kx_server_session_keys(
            &self,
            server_rx: &mut Box<dyn Buffer>,
            server_tx: &mut Box<dyn Buffer>,
            server_pk: &Box<dyn Buffer>,
            server_sk: &Box<dyn Buffer>,
            client_pk: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if server_rx.len() != self.kx_session_key_bytes() {
                return Err(CryptoError::BadRxSessionKeySize);
            }

            if server_tx.len() != self.kx_session_key_bytes() {
                return Err(CryptoError::BadTxSessionKeySize);
            }

            if server_pk.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            if server_sk.len() != self.kx_secret_key_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            if client_pk.len() != self.kx_public_key_bytes() {
                return Err(CryptoError::BadPublicKeySize);
            }

            server_rx.write(0, &server_pk.read_lock()[..4])?;
            server_rx.write(4, &client_pk.read_lock()[..4])?;
            server_tx.write(0, &client_pk.read_lock()[..4])?;
            server_tx.write(4, &server_pk.read_lock()[..4])?;

            Ok(())
        }

        fn aead_nonce_bytes(&self) -> usize {
            8
        }

        fn aead_auth_bytes(&self) -> usize {
            8
        }

        fn aead_secret_bytes(&self) -> usize {
            8
        }

        fn aead_encrypt(
            &self,
            cipher: &mut Box<dyn Buffer>,
            message: &Box<dyn Buffer>,
            adata: Option<&Box<dyn Buffer>>,
            nonce: &Box<dyn Buffer>,
            secret: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if cipher.len() != message.len() + self.aead_auth_bytes() {
                return Err(CryptoError::BadCipherSize);
            }

            if nonce.len() != self.aead_nonce_bytes() {
                return Err(CryptoError::BadNonceSize);
            }

            if secret.len() != self.aead_secret_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            // the goal is to be able to validate that we "encrypted"
            // the message with given secret, nonce, and adata
            // we're just going to store two bytes of each of these
            // then the unencrypted message
            // validations will have a chance of false positive
            // for each out of 2^16

            // zero out the cipher buffer
            cipher.zero();
            // store two bytes of nonce here
            cipher.write(2, &nonce.read_lock()[..2])?;
            // store two bytes of secret here
            cipher.write(4, &secret.read_lock()[..2])?;

            if let Some(adata) = adata {
                // if we have adata, store two bytes of it here
                cipher.write(6, &adata.read_lock()[..2])?;
            }

            // write our full message out here
            cipher.write(8, &message.read_lock())?;

            Ok(())
        }

        fn aead_decrypt(
            &self,
            message: &mut Box<dyn Buffer>,
            cipher: &Box<dyn Buffer>,
            adata: Option<&Box<dyn Buffer>>,
            nonce: &Box<dyn Buffer>,
            secret: &Box<dyn Buffer>,
        ) -> CryptoResult<()> {
            if message.len() != cipher.len() - self.aead_auth_bytes() {
                return Err(CryptoError::BadMessageSize);
            }

            if nonce.len() != self.aead_nonce_bytes() {
                return Err(CryptoError::BadNonceSize);
            }

            if secret.len() != self.aead_secret_bytes() {
                return Err(CryptoError::BadSecretKeySize);
            }

            let cipher = cipher.read_lock();

            // check that this "cipher" used this nonce
            if &cipher[2..4] != &nonce.read_lock()[..2] {
                return Err(CryptoError::CouldNotDecrypt);
            }

            // check that this "cipher" used this secret
            if &cipher[4..6] != &secret.read_lock()[..2] {
                return Err(CryptoError::CouldNotDecrypt);
            }

            if let Some(adata) = adata {
                // check that this "cipher" used this adata
                if &cipher[6..8] != &adata.read_lock()[..2] {
                    return Err(CryptoError::CouldNotDecrypt);
                }
            }

            // return the unencrypted cipher
            message.write(0, &cipher[8..])?;

            Ok(())
        }
    }
}
