use lib3h_protocol::{
    Address,
};

lazy_static! {
    pub static ref NETWORK_A_ID: String = "net_A".to_string();
    pub static ref ALEX_AGENT_ID: Address = "alex".to_string().into_bytes();
    pub static ref BILLY_AGENT_ID: Address = "billy".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_B: Address = "SPACE_B".to_string().into_bytes();
}

//--------------------------------------------------------------------------------------------------
// Predicates
//--------------------------------------------------------------------------------------------------

/// Check if JsonProtocol is of type $p
macro_rules! one_is {
    ($p:pat) => {
        |d| {
            if let $p = d {
                return true;
            }
            return false;
        }
    };
}

/// Check if JsonProtocol is of type $p and meets conditions set in $code
macro_rules! one_is_where {
    ($p:pat, $code:tt) => {
        move |d| return if let $p = d { $code } else { false }
    };
}

#[allow(unused_macros)]
macro_rules! one_let {
    ($p:pat = $enum:ident $code:tt) => {
        if let $p = $enum {
            $code
        } else {
            unimplemented!();
        }
    };
}

