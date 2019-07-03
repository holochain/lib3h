use lib3h_protocol::Address;

lazy_static! {
    pub static ref NETWORK_A_ID: String = "net_A".to_string();
    pub static ref ALEX_AGENT_ID: Address = "alex".to_string().into_bytes();
    pub static ref BILLY_AGENT_ID: Address = "billy".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_B: Address = "SPACE_B".to_string().into_bytes();
}
