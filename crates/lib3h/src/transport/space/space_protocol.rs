#[derive(Debug, Clone, PartialEq)]
pub enum SpaceProtocol {
    Gossip,
    SendDirectMessage,
    FetchData,
    FetchDataResponse,
    // FIXME
}