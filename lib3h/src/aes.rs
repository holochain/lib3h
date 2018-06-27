use openssl;
use rand;
use rand::Rng;

fn get_rand_12() -> [u8; 12] {
    let mut rng = rand::thread_rng();

    let mut arr = [0u8; 12];
    rng.fill(&mut arr[..]);
    arr
}

pub fn gen_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();

    let mut arr = [0u8; 32];
    rng.fill(&mut arr[..]);
    arr
}

pub fn enc(data: &[u8], psk: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let cipher = openssl::symm::Cipher::aes_256_gcm();
    let iv = get_rand_12();
    let mut auth_tag = [0u8; 16];

    let ctext =
        openssl::symm::encrypt_aead(cipher, psk, Some(&iv), &[0u8; 0], data, &mut auth_tag)?;

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&iv);
    out.extend_from_slice(&auth_tag);
    out.extend_from_slice(&ctext);
    Ok(out)
}

pub fn dec(data: &[u8], psk: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let cipher = openssl::symm::Cipher::aes_256_gcm();
    let iv = &data[..12];
    let auth_tag = &data[12..28];

    let out = openssl::symm::decrypt_aead(cipher, psk, Some(iv), &[0u8; 0], &data[28..], auth_tag)?;

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pass() {
        let psk = gen_key();
        let test = String::from("hello");

        let ctext = enc(test.as_bytes(), &psk).unwrap();
        let dtext = dec(&ctext, &psk).unwrap();

        assert_eq!(String::from_utf8_lossy(&dtext), String::from("hello"));
    }
}
