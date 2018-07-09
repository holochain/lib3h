use crypto;
use error;
use hash;
use rmp_serde;
use util;

use rust_base58::base58::ToBase58;
use serde::Serialize;

#[derive(Debug)]
pub enum IdentityError {
    PublicKeysNotInitialized,
    PublicKeyTypeNotFound,
    PrivateKeysNotInitialized,
    PrivateKeyTypeNotFound,
}

fn e_no_pub() -> error::Error {
    error::Error::generic_error(Box::new(IdentityError::PublicKeysNotInitialized))
}

fn e_no_priv() -> error::Error {
    error::Error::generic_error(Box::new(IdentityError::PrivateKeysNotInitialized))
}

fn e_pub_not_found() -> error::Error {
    error::Error::generic_error(Box::new(IdentityError::PublicKeyTypeNotFound))
}

fn e_priv_not_found() -> error::Error {
    error::Error::generic_error(Box::new(IdentityError::PrivateKeyTypeNotFound))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AsymmetricPubKey {
    Rsa4096(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AsymmetricPrivKey {
    Rsa4096(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BundlePubIdentity {
    pub pub_keys: Vec<AsymmetricPubKey>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BundlePrivIdentity {
    pub priv_keys: Vec<AsymmetricPrivKey>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FullIdentity {
    pub pub_raw: Option<Vec<u8>>,
    pub pub_keys: Option<BundlePubIdentity>,
    pub id_hash: Option<Vec<u8>>,
    pub u32_tag: Option<u32>,

    pub priv_raw: Option<Vec<u8>>,
    pub priv_keys: Option<BundlePrivIdentity>,
}

impl FullIdentity {
    pub fn new() -> error::Result<Self> {
        Ok(FullIdentity {
            pub_raw: None,
            pub_keys: None,
            id_hash: None,
            u32_tag: None,

            priv_raw: None,
            priv_keys: None,
        })
    }

    pub fn new_generate(passphrase: &[u8]) -> error::Result<Self> {
        let mut id = FullIdentity::new()?;

        let key_pair = crypto::rsa::gen_key()?;

        id.load_pub_bundle(BundlePubIdentity {
            pub_keys: vec![AsymmetricPubKey::Rsa4096(key_pair.pub_key)],
        })?;

        id.load_priv_bundle(
            BundlePrivIdentity {
                priv_keys: vec![AsymmetricPrivKey::Rsa4096(key_pair.priv_key)],
            },
            passphrase,
        )?;

        Ok(id)
    }

    pub fn get_id_base58(&self) -> error::Result<String> {
        let id_hash = match self.id_hash.as_ref() {
            Some(v) => v,
            None => return Err(e_no_pub()),
        };

        Ok(id_hash.to_base58())
    }

    pub fn load_pub_raw(&mut self, pub_raw: &[u8]) -> error::Result<()> {
        self.pub_raw = Some(pub_raw.to_vec());

        self.pub_keys = Some(BundlePubIdentity { pub_keys: vec![] });

        {
            let raw = match self.pub_raw.as_ref() {
                Some(v) => v,
                None => return Err(e_no_pub()),
            };
            let res = rmp_serde::from_slice(raw)?;
            self.pub_keys = Some(res);
        }

        self.gen_id()?;

        Ok(())
    }

    pub fn load_pub_bundle(&mut self, pub_keys: BundlePubIdentity) -> error::Result<()> {
        self.pub_keys = Some(pub_keys);

        self.pub_raw = Some(Vec::new());

        {
            let pub_raw = match self.pub_raw.as_mut() {
                Some(v) => v,
                None => return Err(e_no_pub()),
            };
            self.pub_keys
                .serialize(&mut rmp_serde::Serializer::new(pub_raw))?;
        }

        self.gen_id()?;

        Ok(())
    }

    fn gen_id(&mut self) -> error::Result<()> {
        let pub_raw = match self.pub_raw.as_ref() {
            Some(v) => v,
            None => return Err(e_no_pub()),
        };
        let mut raw_hash = hash::hash(pub_raw)?;

        // get our u32_tag BEFORE adding multihash tag
        // otherwise we won't be evenly distributed
        self.u32_tag = Some(util::u32_tag_for_hash(&raw_hash)?);

        // claiming byte 136 (0x88) for sha3-512 -> sha2-256 multihash
        let mut multihash: Vec<u8> = vec![136_u8];
        multihash.append(&mut raw_hash);
        self.id_hash = Some(multihash);

        Ok(())
    }

    pub fn load_priv_raw(&mut self, priv_raw: &[u8], passphrase: &[u8]) -> error::Result<()> {
        self.priv_raw = Some(priv_raw.to_vec());
        let psk = hash::hash(passphrase)?;

        self.priv_keys = Some(BundlePrivIdentity { priv_keys: vec![] });

        let priv_dec = crypto::aes::dec(&self.priv_raw.as_ref().unwrap(), &psk)?;
        let res = rmp_serde::from_slice(&priv_dec)?;
        self.priv_keys = Some(res);

        Ok(())
    }

    pub fn load_priv_bundle(
        &mut self,
        priv_keys: BundlePrivIdentity,
        passphrase: &[u8],
    ) -> error::Result<()> {
        self.priv_keys = Some(priv_keys);
        let psk = hash::hash(passphrase)?;

        let mut priv_dec = Vec::new();
        self.priv_keys
            .serialize(&mut rmp_serde::Serializer::new(&mut priv_dec))?;

        let priv_raw = crypto::aes::enc(&priv_dec, &psk)?;
        self.priv_raw = Some(priv_raw);

        Ok(())
    }

    pub fn rsa4096_public_encrypt(&self, data: &[u8]) -> error::Result<Vec<u8>> {
        let pub_keys = match self.pub_keys.as_ref() {
            Some(ref v) => &v.pub_keys,
            None => return Err(e_no_pub()),
        };
        for pub_key in pub_keys {
            match pub_key {
                AsymmetricPubKey::Rsa4096(ref k) => {
                    return Ok(crypto::rsa::enc(data, k)?);
                }
            }
        }
        Err(e_pub_not_found())
    }

    pub fn rsa4096_private_decrypt(&self, data: &[u8]) -> error::Result<Vec<u8>> {
        let priv_keys = match self.priv_keys.as_ref() {
            Some(ref v) => &v.priv_keys,
            None => return Err(e_no_priv()),
        };
        for priv_key in priv_keys {
            match priv_key {
                AsymmetricPrivKey::Rsa4096(ref k) => {
                    return Ok(crypto::rsa::dec(data, k)?);
                }
            }
        }
        Err(e_priv_not_found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static PASSPHRASE: &'static [u8] = b"this is a test passphrase";
    static PUB: &'static [u8] = include_bytes!("test_fixtures/rsa_pub_raw_1");
    static PUB_PACK: &'static [u8] = include_bytes!("test_fixtures/rsa_pub_pack_1");
    static PRIV: &'static [u8] = include_bytes!("test_fixtures/rsa_priv_raw_1");
    static PRIV_PACK: &'static [u8] = include_bytes!("test_fixtures/rsa_priv_pack_1");

    #[test]
    fn it_cannot_get_id_if_no_pub_keys() {
        let id = FullIdentity::new().unwrap();
        let err = id.get_id_base58().unwrap_err();
        assert_eq!("PublicKeysNotInitialized", format!("{}", err));
    }

    #[test]
    fn it_can_gen_pub_id_from_keys() {
        let bundle_pub = BundlePubIdentity {
            pub_keys: vec![AsymmetricPubKey::Rsa4096(PUB.to_vec())],
        };

        let mut id = FullIdentity::new().unwrap();
        id.load_pub_bundle(bundle_pub).unwrap();

        assert_eq!(
            "hg6T5F85q6qbh2hNERT3Z7XgMYMeSzoqvKHm6Rk9Z1Wjy",
            String::from(id.get_id_base58().unwrap())
        );

        assert_eq!(2325535360, id.u32_tag.unwrap());
    }

    #[test]
    fn it_can_gen_pub_id_from_raw() {
        let mut id = FullIdentity::new().unwrap();
        id.load_pub_raw(PUB_PACK).unwrap();

        assert_eq!(
            "hg6T5F85q6qbh2hNERT3Z7XgMYMeSzoqvKHm6Rk9Z1Wjy",
            String::from(id.get_id_base58().unwrap())
        );
        assert_eq!(2325535360, id.u32_tag.unwrap());
    }

    #[test]
    fn it_can_gen_priv_id_from_keys() {
        let bundle_priv = BundlePrivIdentity {
            priv_keys: vec![AsymmetricPrivKey::Rsa4096(PRIV.to_vec())],
        };

        let mut id = FullIdentity::new().unwrap();
        id.load_priv_bundle(bundle_priv, PASSPHRASE).unwrap();

        // to actually test in this direction we need to manually decrypt it
        let psk = hash::hash(PASSPHRASE).unwrap();
        let priv_dec = crypto::aes::dec(&id.priv_raw.as_ref().unwrap(), &psk).unwrap();
        let priv_dec: BundlePrivIdentity = rmp_serde::from_slice(&priv_dec).unwrap();

        match priv_dec.priv_keys[0] {
            AsymmetricPrivKey::Rsa4096(ref k) => {
                assert_eq!(k, &PRIV.to_vec());
            }
        }
    }

    #[test]
    fn it_can_gen_priv_id_from_raw() {
        let mut id = FullIdentity::new().unwrap();
        id.load_priv_raw(PRIV_PACK, PASSPHRASE).unwrap();

        match id.priv_keys.unwrap().priv_keys[0] {
            AsymmetricPrivKey::Rsa4096(ref k) => {
                assert_eq!(k, &PRIV.to_vec());
            }
        }
    }

    #[test]
    fn it_can_generate_and_use() {
        let id = FullIdentity::new_generate(PASSPHRASE).unwrap();

        let enc = id.rsa4096_public_encrypt(b"hello").unwrap();
        let dec = id.rsa4096_private_decrypt(&enc).unwrap();

        assert_eq!(b"hello".to_vec(), dec);
    }
}
