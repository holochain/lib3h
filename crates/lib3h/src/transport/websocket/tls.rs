#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TlsCertificate {
    pub(in crate::transport::websocket) pkcs12_data: Vec<u8>,
    pub(in crate::transport::websocket) passphrase: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TlsConfig {
    Unencrypted,
    FakeServer,
    SuppliedCertificate(TlsCertificate),
}
