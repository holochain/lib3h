use crate::dht::PeerAddress;
use lib3h_protocol::{data_types::EntryData, Address};
use url::Url;

pub type FromPeerAddress = PeerAddress;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct RemoteGossipBundleData {
    pub from_peer_address: PeerAddress,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipToData {
    pub peer_address_list: Vec<PeerAddress>,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct PeerData {
    pub peer_address: PeerAddress,
    #[serde(with = "url_serde")]
    pub peer_uri: Url,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryData {
    pub msg_id: String,
    pub entry_address: Address,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryResponseData {
    pub msg_id: String,
    pub entry: EntryData,
}
