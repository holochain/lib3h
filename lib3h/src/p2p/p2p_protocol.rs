use lib3h_protocol::Lib3hResult;

/// Enum holding all message types in the 'network module <-> network module' protocol.
/// TODO
#[derive(Debug, Clone, PartialEq)]
pub enum P2pProtocol {
    Gossip,
    DirectMessage,
    FetchData,
    FetchDataResponse,
    // FIXME
}

impl P2pProtocol {
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            P2pProtocol::Gossip => vec![0],
            P2pProtocol::DirectMessage => vec![1],
            P2pProtocol::FetchData => vec![2],
            P2pProtocol::FetchDataResponse => vec![3],
        }
    }

    pub fn deserialize(data: &Vec<u8>) -> Lib3hResult<Self> {
        if data.len() != 1 {
            return Err(format_err!("Bad input for deserializing P2pProtocol"));
        }
        let protocol = match data[0] {
            0 => P2pProtocol::Gossip,
            1 => P2pProtocol::DirectMessage,
            2 => P2pProtocol::FetchData,
            3 => P2pProtocol::FetchDataResponse,
            _ => unreachable!(),
        };
        Ok(protocol)
    }
}
