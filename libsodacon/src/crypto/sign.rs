use error;
use crypto::rand::rand_bytes;

use sodiumoxide::crypto::sign::ed25519 as so_sign;

pub fn gen_seed () -> error::Result<Vec<u8>> {
    Ok(rand_bytes(so_sign::SEEDBYTES)?)
}

pub fn keypair_from_seed (seed: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>)> {
    let seed = match so_sign::Seed::from_slice(seed) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid seed")),
    };
    let (sign_pub, sign_priv) = so_sign::keypair_from_seed(&seed);
    Ok((sign_pub.0.to_vec(), sign_priv.0.to_vec()))
}

pub fn sign (data: &[u8], priv_key: &[u8]) -> error::Result<Vec<u8>> {
    let priv_key = match so_sign::SecretKey::from_slice(priv_key) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid privkey")),
    };
    Ok(so_sign::sign_detached(data, &priv_key).0.to_vec())
}

pub fn verify (data: &[u8], signature: &[u8], pub_key: &[u8]) -> error::Result<bool> {
    let pub_key = match so_sign::PublicKey::from_slice(pub_key) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid pubkey")),
    };
    let signature = match so_sign::Signature::from_slice(signature) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid signature")),
    };
    Ok(so_sign::verify_detached(&signature, data, &pub_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_verify_good() {
        let seed = gen_seed().unwrap();
        let (sign_pub, sign_priv) = keypair_from_seed(&seed).unwrap();
        let sig = sign(b"hello", &sign_priv).unwrap();
        assert!(verify(b"hello", &sig, &sign_pub).unwrap());
    }

    #[test]
    #[should_panic]
    fn it_does_not_verify_bad() {
        let seed = gen_seed().unwrap();
        let (sign_pub, sign_priv) = keypair_from_seed(&seed).unwrap();
        let sig = sign(b"hello", &sign_priv).unwrap();
        assert!(verify(b"hello1", &sig, &sign_pub).unwrap());
    }
}
