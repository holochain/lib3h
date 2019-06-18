use lib3h_crypto_api::{Buffer, CryptoError, CryptoResult, CryptoSignature};

use crate::{check_init, crypto_system::SodiumCryptoSystem};

impl CryptoSignature for SodiumCryptoSystem {
    const SIGN_SEED_BYTES: usize = rust_sodium_sys::crypto_sign_SEEDBYTES as usize;
    const SIGN_PUBLIC_KEY_BYTES: usize = rust_sodium_sys::crypto_sign_PUBLICKEYBYTES as usize;
    const SIGN_SECRET_KEY_BYTES: usize = rust_sodium_sys::crypto_sign_SECRETKEYBYTES as usize;
    const SIGN_BYTES: usize = rust_sodium_sys::crypto_sign_BYTES as usize;

    fn sign_seed_keypair<SeedBuffer: Buffer, PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        seed: &SeedBuffer,
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()> {
        check_init();

        if seed.len() != SodiumCryptoSystem::SIGN_SEED_BYTES {
            return Err(CryptoError::BadSeedSize);
        }

        if public_key.len() != SodiumCryptoSystem::SIGN_PUBLIC_KEY_BYTES {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != SodiumCryptoSystem::SIGN_SECRET_KEY_BYTES {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();
        let seed = seed.read_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_seed_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
                raw_ptr_char_immut!(seed),
            );
        }

        Ok(())
    }

    fn sign_keypair<PublicKeyBuffer: Buffer, SecretKeyBuffer: Buffer>(
        public_key: &mut PublicKeyBuffer,
        secret_key: &mut SecretKeyBuffer,
    ) -> CryptoResult<()> {
        check_init();

        if public_key.len() != SodiumCryptoSystem::SIGN_PUBLIC_KEY_BYTES {
            return Err(CryptoError::BadPublicKeySize);
        }

        if secret_key.len() != SodiumCryptoSystem::SIGN_SECRET_KEY_BYTES {
            return Err(CryptoError::BadSecretKeySize);
        }

        let mut public_key = public_key.write_lock();
        let mut secret_key = secret_key.write_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_keypair(
                raw_ptr_char!(public_key),
                raw_ptr_char!(secret_key),
            );
        }

        Ok(())
    }

    fn sign<SignatureBuffer: Buffer, MessageBuffer: Buffer, SecretKeyBuffer: Buffer>(
        signature: &mut SignatureBuffer,
        message: &MessageBuffer,
        secret_key: &SecretKeyBuffer,
    ) -> CryptoResult<()> {
        check_init();

        if signature.len() != SodiumCryptoSystem::SIGN_BYTES {
            return Err(CryptoError::BadSignatureSize);
        }

        if secret_key.len() != SodiumCryptoSystem::SIGN_SECRET_KEY_BYTES {
            return Err(CryptoError::BadSecretKeySize);
        }

        let message = message.read_lock();
        let secret_key = secret_key.read_lock();
        let mut signature = signature.write_lock();

        unsafe {
            rust_sodium_sys::crypto_sign_detached(
                raw_ptr_char!(signature),
                std::ptr::null_mut(),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(secret_key),
            );
        }

        Ok(())
    }

    fn sign_verify<SignatureBuffer: Buffer, MessageBuffer: Buffer, PublicKeyBuffer: Buffer>(
        signature: &SignatureBuffer,
        message: &MessageBuffer,
        public_key: &PublicKeyBuffer,
    ) -> CryptoResult<bool> {
        check_init();

        if signature.len() != SodiumCryptoSystem::SIGN_BYTES {
            return Err(CryptoError::BadSignatureSize);
        }

        if public_key.len() != SodiumCryptoSystem::SIGN_PUBLIC_KEY_BYTES {
            return Err(CryptoError::BadPublicKeySize);
        }

        Ok(unsafe {
            rust_sodium_sys::crypto_sign_verify_detached(
                raw_ptr_char_immut!(signature),
                raw_ptr_char_immut!(message),
                message.len() as libc::c_ulonglong,
                raw_ptr_char_immut!(public_key),
            )
        } == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto_system::{SecureBuffer, SodiumCryptoSystem};
    use lib3h_crypto_api::{CryptoRandom, CryptoSystem};

    #[test]
    fn it_should_sign_and_verify() {
        let mut message: Vec<u8> = vec![0; 8];
        SodiumCryptoSystem::randombytes_buf(&mut message).unwrap();

        let mut seed = SecureBuffer::new(SodiumCryptoSystem::SIGN_SEED_BYTES).unwrap();
        SodiumCryptoSystem::randombytes_buf(&mut seed).unwrap();

        let mut pub_key: Vec<u8> = vec![0; SodiumCryptoSystem::SIGN_PUBLIC_KEY_BYTES];
        let mut priv_key = SecureBuffer::new(SodiumCryptoSystem::SIGN_SECRET_KEY_BYTES).unwrap();

        SodiumCryptoSystem::sign_seed_keypair(&seed, &mut pub_key, &mut priv_key).unwrap();

        let mut sig: Vec<u8> = vec![0; SodiumCryptoSystem::SIGN_BYTES];

        assert!(!SodiumCryptoSystem::sign_verify(&sig, &message, &pub_key).unwrap());

        SodiumCryptoSystem::sign(&mut sig, &message, &priv_key).unwrap();

        assert!(SodiumCryptoSystem::sign_verify(&sig, &message, &pub_key).unwrap());
    }
}
