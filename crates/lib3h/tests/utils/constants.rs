use holochain_persistence_api::hash::HashString;
use lib3h_protocol::Address;
use multihash::Hash;

lazy_static! {
    /// Networks
    pub static ref NETWORK_A_ID: String = "net_A".to_string();
    /// Agents
    pub static ref ALEX_AGENT_ID: Address = "alex".to_string().into_bytes();
    pub static ref BILLY_AGENT_ID: Address = "billy".to_string().into_bytes();
    pub static ref CAMILLE_AGENT_ID: Address = "camille".to_string().into_bytes();
    /// Spaces
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_B: Address = "SPACE_B".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_C: Address = "SPACE_C".to_string().into_bytes();
    /// Entries
    pub static ref ENTRY_ADDRESS_1: Address = "entry_addr_1".to_string().into_bytes();
    pub static ref ENTRY_ADDRESS_2: Address = "entry_addr_2".to_string().into_bytes();
    pub static ref ENTRY_ADDRESS_3: Address = "entry_addr_3".to_string().into_bytes();
    /// Aspects
    pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
    pub static ref ASPECT_ADDRESS_1: Address = generate_address(&*ASPECT_CONTENT_1);
    pub static ref ASPECT_ADDRESS_2: Address = generate_address(&*ASPECT_CONTENT_2);
    pub static ref ASPECT_ADDRESS_3: Address = generate_address(&*ASPECT_CONTENT_3);
}

//--------------------------------------------------------------------------------------------------
// Generators
//--------------------------------------------------------------------------------------------------

pub fn generate_address(content: &[u8]) -> Address {
    HashString::encode_from_bytes(content, Hash::SHA2256)
        .to_string()
        .as_bytes()
        .to_vec()
}

#[allow(dead_code)]
pub fn generate_agent_id(i: u32) -> String {
    format!("agent_{}", i)
}

#[allow(dead_code)]
pub fn generate_space_address(i: u32) -> Address {
    format!("SPACE_{}", i).as_bytes().to_vec()
}

#[allow(dead_code)]
pub fn generate_entry(i: u32) -> (Address, Vec<u8>) {
    let address = format!("entry_addr_{}", i);
    let content = format!("hello-{}", i);
    (address.into(), content.as_bytes().to_vec())
}
