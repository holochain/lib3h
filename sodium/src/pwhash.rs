//! This module provides access to libsodium

use super::{check_init, secbuf::SecBuf};
use lib3h_crypto_api::CryptoError;

pub const OPSLIMIT_INTERACTIVE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_INTERACTIVE as u64;
pub const MEMLIMIT_INTERACTIVE: usize =
    rust_sodium_sys::crypto_pwhash_MEMLIMIT_INTERACTIVE as usize;
pub const OPSLIMIT_MODERATE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_MODERATE as u64;
pub const MEMLIMIT_MODERATE: usize = rust_sodium_sys::crypto_pwhash_MEMLIMIT_MODERATE as usize;
pub const OPSLIMIT_SENSITIVE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_SENSITIVE as u64;
pub const MEMLIMIT_SENSITIVE: usize = rust_sodium_sys::crypto_pwhash_MEMLIMIT_SENSITIVE as usize;

pub const ALG_ARGON2I13: i8 = rust_sodium_sys::crypto_pwhash_ALG_ARGON2I13 as i8;
pub const ALG_ARGON2ID13: i8 = rust_sodium_sys::crypto_pwhash_ALG_ARGON2ID13 as i8;

pub const HASHBYTES: usize = 32 as usize;
pub const SALTBYTES: usize = rust_sodium_sys::crypto_pwhash_SALTBYTES as usize;

/// Calculate a password hash
///
/// @param {SecBuf} password - the password to hash
///
/// @param {u64} opslimit - operation scaling for hashing algorithm
///
/// @param {usize} memlimit - memory scaling for hashing algorithm
///
/// @param {i8} algorithm - which hashing algorithm
///
/// @param {SecBuf} salt - predefined salt (randomized it if you dont want to generate it )
///
/// @param {SecBuf} hash - the hash generated
pub fn hash(
    password: &mut SecBuf,
    ops_limit: u64,
    mem_limit: usize,
    alg: i8,
    salt: &mut SecBuf,
    hash: &mut SecBuf,
) -> Result<(), CryptoError> {
    check_init();
    let salt = salt.read_lock();
    let password = password.read_lock();
    let mut hash = hash.write_lock();
    let hash_len = hash.len() as libc::c_ulonglong;
    let pw_len = password.len() as libc::c_ulonglong;
    let res = unsafe {
        rust_sodium_sys::crypto_pwhash(
            raw_ptr_char!(hash),
            hash_len,
            raw_ptr_ichar_immut!(password),
            pw_len,
            raw_ptr_char_immut!(salt),
            ops_limit as libc::c_ulonglong,
            mem_limit,
            libc::c_int::from(alg),
        )
    };
    match res {
        0 => Ok(()),
        -1 => Err(CryptoError::OutOfMemory),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_generate_with_random_salt() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw1_hash = SecBuf::with_secure(HASHBYTES);
        let mut random_salt = SecBuf::with_insecure(SALTBYTES);
        password.randomize();
        random_salt.randomize();
        hash(
            &mut password,
            OPSLIMIT_INTERACTIVE,
            MEMLIMIT_INTERACTIVE,
            ALG_ARGON2ID13,
            &mut random_salt,
            &mut pw1_hash,
        )
        .unwrap();
        assert_eq!(HASHBYTES, password.len());
    }

    #[test]
    fn it_should_generate_with_salt() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw2_hash = SecBuf::with_secure(HASHBYTES);
        {
            let mut password = password.write_lock();
            password[0] = 42;
            password[1] = 222;
        }
        let mut salt = SecBuf::with_insecure(SALTBYTES);
        hash(
            &mut password,
            OPSLIMIT_INTERACTIVE,
            MEMLIMIT_INTERACTIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw2_hash,
        )
        .unwrap();
        let pw2_hash = pw2_hash.read_lock();
        assert_eq!("[243, 52, 246, 116, 155, 113, 127, 79, 150, 21, 250, 222, 215, 252, 119, 37, 34, 141, 76, 32, 99, 33, 241, 45, 187, 121, 83, 31, 108, 28, 160, 7]",  format!("{:?}", *pw2_hash));
    }

    #[test]
    fn it_should_generate_consistantly() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw1_hash = SecBuf::with_secure(HASHBYTES);
        let mut pw2_hash = SecBuf::with_secure(HASHBYTES);
        password.randomize();
        let mut salt = SecBuf::with_insecure(SALTBYTES);
        salt.randomize();
        hash(
            &mut password,
            OPSLIMIT_INTERACTIVE,
            MEMLIMIT_INTERACTIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw1_hash,
        )
        .unwrap();
        hash(
            &mut password,
            OPSLIMIT_INTERACTIVE,
            MEMLIMIT_INTERACTIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw2_hash,
        )
        .unwrap();
        let pw1_hash = pw1_hash.read_lock();
        let pw2_hash = pw2_hash.read_lock();
        assert_eq!(format!("{:?}", *pw1_hash), format!("{:?}", *pw2_hash));
    }
}
