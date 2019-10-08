use holochain_persistence_api::hash::HashString;
use lib3h::engine::GatewayId;
use lib3h_protocol::types::{AgentPubKey, AspectHash, EntryHash, SpaceHash};
use multihash::Hash;
use std::sync::{Arc, Mutex};

lazy_static! {
    /// Networks
    pub static ref NETWORK_A_ID: NetworkHash = "net_A".into();
    /// Agents
    pub static ref ALEX_AGENT_ID: AgentPubKey = "alex".into();
    pub static ref BILLY_AGENT_ID: AgentPubKey = "billy".into();
    pub static ref CAMILLE_AGENT_ID: AgentPubKey = "camille".into();
    /// Spaces
    pub static ref SPACE_ADDRESS_A: SpaceHash = "appA".into();
    pub static ref SPACE_ADDRESS_B: SpaceHash = "appB".into();
    pub static ref SPACE_ADDRESS_C: SpaceHash = "appC".into();
    /// Entries
    pub static ref ENTRY_ADDRESS_1: EntryHash = "entry_addr_1".into();
    pub static ref ENTRY_ADDRESS_2: EntryHash = "entry_addr_2".into();
    pub static ref ENTRY_ADDRESS_3: EntryHash = "entry_addr_3".into();
    /// Aspects
    pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
    pub static ref ASPECT_CONTENT_4: Vec<u8> = "other-4".as_bytes().to_vec();
    pub static ref ASPECT_ADDRESS_1: AspectHash = generate_hash(&*ASPECT_CONTENT_1).into();
    pub static ref ASPECT_ADDRESS_2: AspectHash = generate_hash(&*ASPECT_CONTENT_2).into();
    pub static ref ASPECT_ADDRESS_3: AspectHash = generate_hash(&*ASPECT_CONTENT_3).into();
    pub static ref ASPECT_ADDRESS_4: AspectHash = generate_hash(&*ASPECT_CONTENT_4).into();

    // TODO use port 0 and have transport wss return back actually bound port
    static ref PORT: Arc<Mutex<u32>> = Arc::new(Mutex::new(64528));

    pub static ref MIRROR_NODES_COUNT: u8 = 10;

}

//--------------------------------------------------------------------------------------------------
// Generators
//--------------------------------------------------------------------------------------------------

pub fn generate_hash(content: &[u8]) -> HashString {
    HashString::encode_from_bytes(content, Hash::SHA2256)
}

#[allow(dead_code)]
pub fn generate_agent_id(i: u32) -> String {
    format!("agent_{}", i)
}

#[allow(dead_code)]
pub fn generate_space_address(i: u32) -> SpaceHash {
    format!("app{}", i).as_str().into()
}

#[allow(dead_code)]
pub fn generate_entry(i: u32) -> (EntryHash, Vec<u8>) {
    let address = format!("entry_addr_{}", i);
    let content = format!("hello-{}", i);
    (address.as_str().into(), content.as_bytes().to_vec())
}

#[allow(dead_code)]
pub fn generate_port() -> u32 {
    let mut port = PORT.lock().unwrap();
    *port += 1;
    *port
}
