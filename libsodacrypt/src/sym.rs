use error;
use rand::rand_bytes;

use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305 as so_box;

pub fn gen_random_psk () -> error::Result<Vec<u8>> {
    Ok(rand_bytes(so_box::PRECOMPUTEDKEYBYTES)?)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_decrypt_good() {
        let psk = gen_random_psk().unwrap();
        let (nonce, c) = enc(b"test data", &psk).unwrap();

        assert_eq!(b"test data".to_vec(), dec(&c, &nonce, &psk).unwrap());
    }

    #[test]
    #[should_panic]
    fn it_does_not_decrypt_bad() {
        let psk = gen_random_psk().unwrap();
        let (nonce, mut c) = enc(b"test data", &psk).unwrap();
        c[0] = c[0] + 1;

        assert_eq!(b"test data".to_vec(), dec(&c, &nonce, &psk).unwrap());
    }
}
