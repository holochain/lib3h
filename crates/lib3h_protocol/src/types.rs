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

impl From<&HashString> for SpaceHash {
    fn from(s: &HashString) -> SpaceHash {
        (*s).to_owned().into()
    }
}

impl From<SpaceHash> for HashString {
    fn from(h: SpaceHash) -> HashString {
        h.0
    }
}

impl From<&SpaceHash> for HashString {
    fn from(h: &SpaceHash) -> HashString {
        (*h).to_owned().into()
    }
}

impl From<&str> for SpaceHash {
    fn from(s: &str) -> SpaceHash {
        HashString::from(s).into()
    }
}

impl From<String> for SpaceHash {
    fn from(s: String) -> SpaceHash {
        SpaceHash::from(s.as_str())
    }
}

impl From<&String> for SpaceHash {
    fn from(s: &String) -> SpaceHash {
        (*s).to_owned().into()
    }
}

impl From<SpaceHash> for String {
    fn from(s: SpaceHash) -> String {
        s.hash_string().to_owned().into()
    }
}

impl From<&SpaceHash> for String {
    fn from(s: &SpaceHash) -> String {
        (*s).to_owned().into()
    }
}

impl SpaceHash {
    pub fn new() -> SpaceHash {
        SpaceHash(HashString::new())
    }

    pub fn hash_string(&self) -> &HashString {
        &self.0
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
// NetworkHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct NetworkHash(HashString);

impl fmt::Display for NetworkHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<HashString> for NetworkHash {
    fn from(s: HashString) -> NetworkHash {
        NetworkHash(s)
    }
}

impl From<NetworkHash> for HashString {
    fn from(h: NetworkHash) -> HashString {
        h.0
    }
}

impl<'a> From<&'a HashString> for NetworkHash {
    fn from(s: &HashString) -> NetworkHash {
        NetworkHash::from(s.to_owned())
    }
}

impl<'a> From<&'a str> for NetworkHash {
    fn from(s: &str) -> NetworkHash {
        HashString::from(s.to_owned()).into()
    }
}

impl NetworkHash {
    pub fn new() -> NetworkHash {
        NetworkHash(HashString::new())
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

#[cfg(test)]
pub mod tests {

    use super::SpaceHash;
    use crate::fixture::space_hash_fresh;
    use holochain_persistence_api::{fixture::test_hash_a, hash::HashString};
    use uuid::Uuid;

    #[test]
    fn display_for_space_hash() {
        let s = Uuid::new_v4().to_string();
        let space_hash = SpaceHash::from(HashString::from(s.clone()));

        assert_eq!(s, format!("{}", &space_hash),);
    }

    #[test]
    fn space_hash_from_hash_string() {
        let hash = test_hash_a();

        // cloned
        let space_hash = SpaceHash::from(hash.clone());

        assert_eq!(&hash, space_hash.hash_string());

        // referenced
        let space_hash = SpaceHash::from(&hash);

        assert_eq!(&hash, space_hash.hash_string());
    }

    #[test]
    fn hash_string_from_space_hash() {
        let space_hash = space_hash_fresh();

        // cloned
        let hash_string = HashString::from(space_hash.clone());

        assert_eq!(space_hash.hash_string(), &hash_string,);

        // reference
        let hash_string = HashString::from(&space_hash);

        assert_eq!(space_hash.hash_string(), &hash_string);
    }

    #[test]
    fn space_hash_from_str() {
        let str = "foo";

        let space_hash = SpaceHash::from(str);

        assert_eq!(space_hash.hash_string(), &HashString::from(str),);

        let string = String::from(str);

        // cloned string
        let space_hash = SpaceHash::from(string.clone());

        assert_eq!(String::from(space_hash.hash_string().clone()), string,);

        // reference
        let space_hash = SpaceHash::from(&string);

        assert_eq!(String::from(space_hash.hash_string().clone()), string,);
    }

    #[test]
    fn str_from_space_hash() {
        let s = "foo";
        let space_hash = SpaceHash::from(s);

        // cloned
        assert_eq!(&String::from(s), &String::from(space_hash.clone()),);

        // referenced
        assert_eq!(&String::from(s), &String::from(&space_hash),);
    }

}
