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
