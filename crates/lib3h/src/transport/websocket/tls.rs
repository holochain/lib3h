extern crate native_tls;
extern crate openssl;

use crate::transport::{
    error::TransportResult,
    websocket::{FAKE_PASS, FAKE_PKCS12},
};

use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{self, X509Name, X509},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

// Generates a key/cert pair
fn generate_pair() -> (PKey<Private>, x509::X509) {
    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_nid(openssl::nid::Nid::COMMONNAME, "example.com")
        .unwrap();
    let name = name.build();

    let serial_number = {
        let mut serial = BigNum::new().unwrap();
        serial.rand(159, MsbOption::MAYBE_ZERO, false).unwrap();
        serial.to_asn1_integer().unwrap()
    };

    let mut builder = X509::builder().unwrap();
    builder.set_serial_number(&serial_number).unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    let not_before = Asn1Time::days_from_now(0).unwrap();
    builder.set_not_before(&not_before).unwrap();
    let not_after = Asn1Time::days_from_now(3650).unwrap();
    builder.set_not_after(&not_after).unwrap();
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
    pub fn build_from_entropy() -> Self {
        let (key, cert) = generate_pair();

        let random_passphrase: String = thread_rng().sample_iter(&Alphanumeric).take(30).collect();

        let pkcs12 = openssl::pkcs12::Pkcs12::builder()
            .build(&random_passphrase, "friendly_name", &*key, &cert)
            .unwrap();

        // The DER-encoded bytes of the archive
        let der = pkcs12.to_der().unwrap();

        Self {
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

impl TlsConfig {
    pub fn build_from_entropy() -> Self {
        TlsConfig::SuppliedCertificate(TlsCertificate::build_from_entropy())
    }

    pub fn get_identity(&self) -> TransportResult<native_tls::Identity> {
        Ok(match self {
            TlsConfig::Unencrypted => unimplemented!(),
            TlsConfig::FakeServer => native_tls::Identity::from_pkcs12(FAKE_PKCS12, FAKE_PASS)?,
            TlsConfig::SuppliedCertificate(cert) => {
                native_tls::Identity::from_pkcs12(&cert.pkcs12_data, &cert.passphrase)?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[derive(Debug)]
    struct MockStream {
        name: String,
        recv_bytes: Vec<u8>,
        send_bytes: Vec<u8>,
        should_end: bool,
    }

    impl MockStream {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                recv_bytes: Vec::new(),
                send_bytes: Vec::new(),
                should_end: false,
            }
        }

        pub fn inject_recv(&mut self, bytes: Vec<u8>) {
            self.recv_bytes.extend(bytes);
        }

        pub fn drain_send(&mut self) -> Vec<u8> {
            self.send_bytes.drain(..).collect()
        }

        pub fn set_should_end(&mut self) {
            self.should_end = true;
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            println!("{} got read", self.name);

            if self.recv_bytes.len() == 0 {
                if self.should_end {
                    return Ok(0);
                } else {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }
            }

            let v: Vec<u8> = self
                .recv_bytes
                .drain(0..std::cmp::min(buf.len(), self.recv_bytes.len()))
                .collect();
            buf[0..v.len()].copy_from_slice(&v);
            Ok(v.len())
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            println!("{} got write {}", self.name, buf.len());
            self.send_bytes.extend(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_meta_mock_stream_should_work() {
        let mut s = MockStream::new("test");
        s.write_all(b"test").unwrap();
        assert_eq!("test", &String::from_utf8_lossy(&s.drain_send()[..]));
        s.inject_recv(b"hello".to_vec());
        s.set_should_end();
        let mut v = Vec::new();
        s.read_to_end(&mut v).unwrap();
        assert_eq!("hello", &String::from_utf8_lossy(&v[..]));
    }

    fn test_enc_dec(tls_config: TlsConfig) {
        let connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap();
        let client_side = connector.connect("test.test", MockStream::new("client"));

        // handshake client send 1

        let mut client_side = match client_side {
            Err(native_tls::HandshakeError::WouldBlock(socket)) => socket,
            _ => panic!("unexpected"),
        };

        println!("cs1 {:?}", client_side);

        let mut server_side = MockStream::new("server");
        server_side.inject_recv(client_side.get_mut().drain_send());

        let identity = tls_config.get_identity().unwrap();
        let acceptor = native_tls::TlsAcceptor::new(identity).unwrap();
        let server_side = acceptor.accept(server_side);

        // handshake server send 1

        let mut server_side = match server_side {
            Err(native_tls::HandshakeError::WouldBlock(socket)) => socket,
            _ => panic!("unexpected"),
        };

        println!("ss1 {:?}", server_side);

        client_side
            .get_mut()
            .inject_recv(server_side.get_mut().drain_send());

        // handshake client send 2

        let mut client_side = match client_side.handshake() {
            Err(native_tls::HandshakeError::WouldBlock(socket)) => socket,
            _ => panic!("unexpected"),
        };

        println!("cs2 {:?}", client_side);

        server_side
            .get_mut()
            .inject_recv(client_side.get_mut().drain_send());

        // handshake server send 2 - FINAL

        let mut server_side = match server_side.handshake() {
            Ok(socket) => socket,
            _ => panic!("unexpected"),
        };

        println!("ss2 {:?}", server_side);

        client_side
            .get_mut()
            .inject_recv(server_side.get_mut().drain_send());

        // handshake client send 3 - FINAL

        let mut client_side = match client_side.handshake() {
            Ok(socket) => socket,
            _ => panic!("unexpected"),
        };

        println!("cs3 {:?}", client_side);

        // -- we have now completed handshaking -- //

        // -- try sending a message from client to server -- //

        const TO_SERVER: &'static [u8] = b"test-message-to-server";

        client_side.write_all(TO_SERVER).unwrap();
        let to_server = client_side.get_mut().drain_send();
        assert_ne!(TO_SERVER, &to_server[..]);

        server_side.get_mut().inject_recv(to_server);
        server_side.get_mut().set_should_end();

        let mut from_client = Vec::new();
        server_side.read_to_end(&mut from_client).unwrap();
        assert_eq!(TO_SERVER, &from_client[..]);

        // -- try sending a message from server to client -- //

        const TO_CLIENT: &'static [u8] = b"test-message-to-client";

        server_side.write_all(TO_CLIENT).unwrap();
        let to_client = server_side.get_mut().drain_send();
        assert_ne!(TO_CLIENT, &to_client[..]);

        client_side.get_mut().inject_recv(to_client);
        client_side.get_mut().set_should_end();

        let mut from_server = Vec::new();
        client_side.read_to_end(&mut from_server).unwrap();
        assert_eq!(TO_CLIENT, &from_server[..]);
    }

    #[test]
    fn it_can_use_fake_server_tls() {
        test_enc_dec(TlsConfig::FakeServer);
    }

    #[test]
    fn it_can_use_self_signed_ephemeral_tls() {
        test_enc_dec(TlsConfig::build_from_entropy());
    }
}
