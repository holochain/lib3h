use crate::Address;

/// Tuple holding all the info required for identifying an Aspect.
/// (entry_address, content hash)
pub type AspectKey = (Address, Address);

//--------------------------------------------------------------------------------------------------
// Semi-opaque Holochain Entry
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum EntryAspectKind {
    Content, // the actual entry content
    Header,  // the header for the entry
    Meta,    // could be EntryWithHeader for links
    ValidationResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryAspect {
    pub kind: EntryAspectKind,
    pub publish_ts: u64,
    pub data: String, // opaque, but in core would be EntryWithHeader for both Entry and Meta
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryData {
    pub aspect_list: Vec<EntryAspect>,
    pub entry_address: Address,
}

//--------------------------------------------------------------------------------------------------
// Generic responses
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ResultData {
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
// DHT Entry
//--------------------------------------------------------------------------------------------------

/// Wrapped Entry message
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimedEntryData {
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub entry: EntryData,
}

/// Entry hodled message
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntryStoredData {
    pub space_address: Address,
    pub provider_agent_id: Address,
    pub entry_address: Address,
    pub holder_agent_id: Address,
}

/// Request for Entry
#[derive(Debug, Clone, PartialEq)]
pub struct FetchEntryData {
    pub space_address: Address,
    pub entry_address: Address,
    pub request_id: String,
    pub requester_agent_id: Address,
}

/// DHT data response from a request
#[derive(Debug, Clone, PartialEq)]
pub struct FetchEntryResultData {
    pub request_id: String,
    pub requester_agent_id: Address,
    pub entry: ClaimedEntryData,
}

/// Identifier of what entry (and its meta?) to drop
#[derive(Debug, Clone, PartialEq)]
pub struct DropEntryData {
    pub space_address: Address,
    pub request_id: String,
    pub entry_address: Address,
}

//--------------------------------------------------------------------------------------------------
// Lists (publish & hold)
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct GetListData {
    pub space_address: Address,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryListData {
    pub space_address: Address,
    pub request_id: String,
    pub entry_address_list: Vec<Address>,
}
