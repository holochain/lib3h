use holochain_persistence_api::hash::HashString;
use lib3h_protocol::Address;
use multihash::Hash;
use std::sync::{Arc, Mutex};

lazy_static! {
    /// Networks
    pub static ref NETWORK_A_ID: String = "net_A".to_string();
    /// Agents
    pub static ref ALEX_AGENT_ID: Address = "alex".into();
    pub static ref BILLY_AGENT_ID: Address = "billy".into();
    pub static ref CAMILLE_AGENT_ID: Address = "camille".into();
    /// Spaces
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".into();
    pub static ref SPACE_ADDRESS_B: Address = "SPACE_B".into();
    pub static ref SPACE_ADDRESS_C: Address = "SPACE_C".into();
    /// Entries
    pub static ref ENTRY_ADDRESS_1: Address = "entry_addr_1".into();
    pub static ref ENTRY_ADDRESS_2: Address = "entry_addr_2".into();
    pub static ref ENTRY_ADDRESS_3: Address = "entry_addr_3".into();
    /// Aspects
    pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
    pub static ref ASPECT_ADDRESS_1: Address = generate_address(&*ASPECT_CONTENT_1);
    pub static ref ASPECT_ADDRESS_2: Address = generate_address(&*ASPECT_CONTENT_2);
    pub static ref ASPECT_ADDRESS_3: Address = generate_address(&*ASPECT_CONTENT_3);

    // TODO use port 0 and have transport wss return back actually bound port
    static ref PORT: Arc<Mutex<u32>> = Arc::new(Mutex::new(64528));
}

//--------------------------------------------------------------------------------------------------
// Generators
//--------------------------------------------------------------------------------------------------

pub fn generate_address(content: &[u8]) -> Address {
    HashString::encode_from_bytes(content, Hash::SHA2256)
}

#[allow(dead_code)]
pub fn generate_agent_id(i: u32) -> String {
    format!("agent_{}", i)
}

#[allow(dead_code)]
pub fn generate_space_address(i: u32) -> Address {
    format!("SPACE_{}", i).into()
}

#[allow(dead_code)]
pub fn generate_entry(i: u32) -> (Address, Vec<u8>) {
    let address = format!("entry_addr_{}", i);
    let content = format!("hello-{}", i);
    (address.into(), content.as_bytes().to_vec())
}

#[allow(dead_code)]
pub fn generate_port() -> u32 {
    let mut port = PORT.lock().unwrap();
    *port += 1;
    *port
}
