use holochain_persistence_api::hash::HashString;
use std::fmt;

//--------------------------------------------------------------------------------------------------
// SpaceHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct SpaceHash(HashString);

impl fmt::Display for SpaceHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for SpaceHash {
    fn from(s: HashString) -> SpaceHash {
        SpaceHash(s)
    }
}

impl From<SpaceHash> for HashString {
    fn from(h: SpaceHash) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for SpaceHash {
    fn from(s: &HashString) -> SpaceHash {
        SpaceHash::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for SpaceHash {
    fn from(s: &str) -> SpaceHash {
        HashString::from(s.to_owned()).into()
    }
}

impl SpaceHash {
    pub fn new() -> SpaceHash {
        SpaceHash(HashString::new())
    }
}

//--------------------------------------------------------------------------------------------------
// EntryHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct EntryHash(HashString);

impl fmt::Display for EntryHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for EntryHash {
    fn from(s: HashString) -> EntryHash {
        EntryHash(s)
    }
}

impl From<EntryHash> for HashString {
    fn from(h: EntryHash) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for EntryHash {
    fn from(s: &HashString) -> EntryHash {
        EntryHash::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for EntryHash {
    fn from(s: &str) -> EntryHash {
        HashString::from(s.to_owned()).into()
    }
}

impl EntryHash {
    pub fn new() -> EntryHash {
        EntryHash(HashString::new())
    }
}

//--------------------------------------------------------------------------------------------------
// AspectHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct AspectHash(HashString);

impl fmt::Display for AspectHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for AspectHash {
    fn from(s: HashString) -> AspectHash {
        AspectHash(s)
    }
}

impl From<AspectHash> for HashString {
    fn from(h: AspectHash) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for AspectHash {
    fn from(s: &HashString) -> AspectHash {
        AspectHash::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for AspectHash {
    fn from(s: &str) -> AspectHash {
        HashString::from(s.to_owned()).into()
    }
}

impl AspectHash {
    pub fn new() -> AspectHash {
        AspectHash(HashString::new())
    }
}

//--------------------------------------------------------------------------------------------------
// AgentPubKey: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct AgentPubKey(HashString);

impl fmt::Display for AgentPubKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for AgentPubKey {
    fn from(s: HashString) -> AgentPubKey {
        AgentPubKey(s)
    }
}

impl From<AgentPubKey> for HashString {
    fn from(h: AgentPubKey) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for AgentPubKey {
    fn from(s: &HashString) -> AgentPubKey {
        AgentPubKey::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for AgentPubKey {
    fn from(s: &str) -> AgentPubKey {
        HashString::from(s.to_owned()).into()
    }
}

impl From<String> for AgentPubKey {
    fn from(s: String) -> AgentPubKey {
        HashString::from(s.to_owned()).into()
    }
}

impl AgentPubKey {
    pub fn new() -> AgentPubKey {
        AgentPubKey(HashString::new())
    }
}

//--------------------------------------------------------------------------------------------------
// NodePubKey: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct NodePubKey(HashString);

impl fmt::Display for NodePubKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for NodePubKey {
    fn from(s: HashString) -> NodePubKey {
        NodePubKey(s)
    }
}

impl From<NodePubKey> for HashString {
    fn from(h: NodePubKey) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for NodePubKey {
    fn from(s: &HashString) -> NodePubKey {
        NodePubKey::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for NodePubKey {
    fn from(s: &str) -> NodePubKey {
        HashString::from(s.to_owned()).into()
    }
}

impl NodePubKey {
    pub fn new() -> NodePubKey {
        NodePubKey(HashString::new())
    }
}
