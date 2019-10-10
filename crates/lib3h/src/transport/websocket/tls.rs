extern crate native_tls;
extern crate openssl;

use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{self, X509Name, X509},
}; //, X509NameRef, X509Ref};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

// Generates a key/cert pair
fn generate_pair() -> (PKey<Private>, x509::X509) {
    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_nid(openssl::nid::Nid::COMMONNAME, "example.com")
        .unwrap();
    let name = name.build();
    //let name_ref = X509NameRef::new(&name);

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();

    let cert: openssl::x509::X509 = builder.build();

    (key, cert)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TlsCertificate {
    pub(in crate::transport::websocket) pkcs12_data: Vec<u8>,
    pub(in crate::transport::websocket) passphrase: String,
}

impl TlsCertificate {
    /// Creates a self-signed certificate with an entropy key and passphrase.
    /// This makes it possible to use a TLS encrypted connection securely between two
    /// peers using the lib3h websockt actor.
    pub fn build_from_entropy() -> TlsCertificate {
        let (key, cert) = generate_pair();

        let random_passphrase: String = thread_rng().sample_iter(&Alphanumeric).take(30).collect();

        let pkcs12 = openssl::pkcs12::Pkcs12::builder()
            .build(&random_passphrase, "friendly_name", &*key, &cert)
            .unwrap();

        // The DER-encoded bytes of the archive
        let der = pkcs12.to_der().unwrap();

        TlsCertificate {
            pkcs12_data: der,
            passphrase: random_passphrase,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TlsConfig {
    Unencrypted,
    FakeServer,
    SuppliedCertificate(TlsCertificate),
}
