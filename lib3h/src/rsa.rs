use openssl;

pub struct Key {
    pub pub_key: Vec<u8>,
    pub priv_key: Vec<u8>,
}

pub fn gen_key() -> Result<Key, openssl::error::ErrorStack> {
    let rsa = openssl::rsa::Rsa::generate(4096)?;

    Ok(Key {
        pub_key: rsa.public_key_to_der()?,
        priv_key: rsa.private_key_to_der()?,
    })
}

pub fn enc(data: &[u8], pub_key: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let rsa_enc = openssl::rsa::Rsa::public_key_from_der(pub_key)?;

    let mut ctext = [0u8; 512];

    let size = rsa_enc.public_encrypt(data, &mut ctext, openssl::rsa::Padding::PKCS1_OAEP)?;

    Ok(ctext[..size].to_vec())
}

pub fn dec(data: &[u8], priv_key: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let rsa_dec = openssl::rsa::Rsa::private_key_from_der(priv_key)?;

    let mut dtext = [0u8; 512];

    let size = rsa_dec.private_decrypt(data, &mut dtext, openssl::rsa::Padding::PKCS1_OAEP)?;

    Ok(dtext[..size].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pass() {
        let key = gen_key().unwrap();
        let test = String::from("hello");

        let ctext = enc(test.as_bytes(), &key.pub_key).unwrap();
        let dtext = dec(&ctext, &key.priv_key).unwrap();

        assert_eq!(String::from_utf8_lossy(&dtext), String::from("hello"));
    }
}
