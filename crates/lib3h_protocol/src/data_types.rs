use crate::Address;

/// Tuple holding all the info required for identifying an Aspect.
/// (entry_address, aspect_address)
pub type AspectKey = (Address, Address);

//--------------------------------------------------------------------------------------------------
// Entry (Semi-opaque Holochain entry type)
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct EntryAspectData {
    pub aspect_address: Address,
    pub type_hint: String,
    pub aspect: Vec<u8>,
    pub publish_ts: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryData {
    pub entry_address: Address,
    pub aspect_list: Vec<EntryAspectData>,
}

//--------------------------------------------------------------------------------------------------
// Generic responses
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct GenericResultData {
    pub request_id: String,
    pub space_address: Address,
    pub to_agent_id: Address,
    pub result_info: Vec<u8>,
}

//--------------------------------------------------------------------------------------------------
// Connection
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectData {
    /// Identifier of this request
    pub request_id: String,
    /// A transport address to connect to.
    /// We should find peers at that address.
    /// Ex:
    ///  - `wss://192.168.0.102:58081/`
    ///  - `holorelay://x.x.x.x`
    pub peer_transport: String,
    /// TODO: Add a machine Id?
    /// Specify to which network to connect to.
    /// Empty string for 'any'
    pub network_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedData {
    /// Identifier of the `Connect` request we are responding to
    pub request_id: String,
    /// MachineId of the first peer we are connected to
    pub machine_id: Address,
    // TODO: Add network_id? Or let local client figure it out with the request_id?
    // TODO: Maybe add some info on network state?
    // pub peer_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisconnectedData {
    /// Specify to which network to connect to.
    /// Empty string for 'all'
    pub network_id: String,
}

//--------------------------------------------------------------------------------------------------
// Space tracking
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct SpaceData {
    /// Identifier of this request
    pub request_id: String,
    pub space_address: Address,
    pub agent_id: Address,
}

//--------------------------------------------------------------------------------------------------
// Direct Messaging
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct DirectMessageData {
    pub space_address: Address,
    pub request_id: String,
    pub to_agent_id: Address,
    pub from_agent_id: Address,
    pub content: Vec<u8>,
}

//--------------------------------------------------------------------------------------------------
// Query
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct QueryEntryData {
    pub space_address: Address,
    pub entry_address: Address,
    pub request_id: String,
    pub requester_agent_id: Address,
    pub query: Vec<u8>, // opaque query struct
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryEntryResultData {
    pub space_address: Address,
    pub entry_address: Address,
    pub request_id: String,
    pub requester_agent_id: Address,
    pub responder_agent_id: Address,
    pub query_result: Vec<u8>, // opaque query-result struct
}

//--------------------------------------------------------------------------------------------------
// Publish, Store & Drop
//--------------------------------------------------------------------------------------------------

/// Wrapped Entry message
#[derive(Debug, Clone, PartialEq)]
pub struct ProvidedEntryData {
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub entry: EntryData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreEntryAspectData {
    pub request_id: String,
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub entry_address: Address,
    pub entry_aspect: EntryAspectData,
}

/// Identifier of what entry (and its meta?) to drop
#[derive(Debug, Clone, PartialEq)]
pub struct DropEntryData {
    pub space_address: Address,
    pub request_id: String,
    pub entry_address: Address,
}

//--------------------------------------------------------------------------------------------------
// Gossip
//--------------------------------------------------------------------------------------------------

/// Request for Entry
#[derive(Debug, Clone, PartialEq)]
pub struct FetchEntryData {
    pub space_address: Address,
    pub entry_address: Address,
    pub request_id: String,
    pub provider_agent_id: Address,
    pub aspect_address_list: Option<Vec<Address>>, // None -> Get all, otherwise get specified aspects
}

/// DHT data response from a request
#[derive(Debug, Clone, PartialEq)]
pub struct FetchEntryResultData {
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub request_id: String,
    pub entry: EntryData,
}

//--------------------------------------------------------------------------------------------------
// Lists (publish & hold)
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct GetListData {
    pub space_address: Address,
    /// Request List from a specific Agent
    pub provider_agent_id: Address,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryListData {
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub request_id: String,
    pub address_map: std::collections::HashMap<Address, Vec<Address>>, // Aspect addresses per entry
}
