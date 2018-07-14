/*!
Symmetric key encryption utility functions.

# Examples

## We should be able to encrypt / decrypt bytes.
```
use libsodacrypt::sym::*;

let psk = gen_random_psk().unwrap();
let (nonce, cipher_data) = enc(b"test data", &psk).unwrap();

assert_eq!(b"test data".to_vec(), dec(&cipher_data, &nonce, &psk).unwrap());
```

## If the encrypted data is corrupt, the decrypt step should panic.
```should_panic
use libsodacrypt::sym::*;

let psk = gen_random_psk().unwrap();
let (nonce, mut cipher_data) = enc(b"test data", &psk).unwrap();

cipher_data[0] = cipher_data[0] + 1;
assert_eq!(b"test data".to_vec(), dec(&cipher_data, &nonce, &psk).unwrap());
```
*/
use error;
use rand::rand_bytes;

use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305 as so_box;

/**
Generate a random pre-shared symmetric key, of the correct size.

# Examples

```
use libsodacrypt::sym::gen_random_psk;

let psk = gen_random_psk().unwrap();
```
*/
pub fn gen_random_psk () -> error::Result<Vec<u8>> {
    Ok(rand_bytes(so_box::PRECOMPUTEDKEYBYTES)?)
}

/**
Encrypt data with a pre-existing pre-shared symmetric key.

# Examples

```
use libsodacrypt::sym::*;

let psk = gen_random_psk().unwrap();
let (nonce, cipher_data) = enc(b"hello", &psk).unwrap();
```
*/
pub fn enc (data: &[u8], psk: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>)> {
    if data.len() > 4096 {
        return Err(error::Error::str_error("enc is specd for <= 4096 bytes"));
    }
    let nonce = so_box::gen_nonce();
    let psk = match so_box::PrecomputedKey::from_slice(psk) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid psk")),
    };
    Ok((nonce.0.to_vec(), so_box::seal_precomputed(data, &nonce, &psk)))
}

/**
Decrypt data with a pre-existing pre-shared symmetric key / nonce.

# Examples

```
use libsodacrypt::sym::*;

let psk = gen_random_psk().unwrap();
let (nonce, cipher_data) = enc(b"test data", &psk).unwrap();

assert_eq!(b"test data".to_vec(), dec(&cipher_data, &nonce, &psk).unwrap());
```
*/
pub fn dec (data: &[u8], nonce: &[u8], psk: &[u8]) -> error::Result<Vec<u8>> {
    let nonce = match so_box::Nonce::from_slice(nonce) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid nonce")),
    };
    let psk = match so_box::PrecomputedKey::from_slice(psk) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid psk")),
    };
    match so_box::open_precomputed(&data, &nonce, &psk) {
        Ok(v) => Ok(v),
        Err(_) => Err(error::Error::str_error("failed to decrypt")),
    }
}
