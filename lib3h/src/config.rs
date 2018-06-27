use ident;
use netinfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum IdentityType {
    Ephemeral,
    // Supplied(pub_file: String, priv_file: String, passphrase: Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeConfig {
    pub identity_type: IdentityType,
    pub binding_endpoints: Vec<netinfo::Endpoint>,
    pub bootstrap_endpoints: Vec<netinfo::Endpoint>,
}
