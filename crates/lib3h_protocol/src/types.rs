use holochain_persistence_api::hash::HashString;
use std::fmt;

pub trait HashStringNewType {
    fn hash_string(&self) -> &HashString;
}

//--------------------------------------------------------------------------------------------------
// SpaceHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct SpaceHash(HashString);

impl HashStringNewType for SpaceHash {
    fn hash_string(&self) -> &HashString {
        &self.0
    }
}

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

//--------------------------------------------------------------------------------------------------
// EntryHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(
    Shrinkwrap, PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash,
)]
pub struct EntryHash(HashString);

impl HashStringNewType for EntryHash {
    fn hash_string(&self) -> &HashString {
        &self.0
    }
}

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

impl From<&HashString> for EntryHash {
    fn from(s: &HashString) -> EntryHash {
        (*s).to_owned().into()
    }
}

impl From<EntryHash> for HashString {
    fn from(h: EntryHash) -> HashString {
        h.0
    }
}

impl From<&EntryHash> for HashString {
    fn from(h: &EntryHash) -> HashString {
        (*h).to_owned().into()
    }
}

impl From<&str> for EntryHash {
    fn from(s: &str) -> EntryHash {
        HashString::from(s).into()
    }
}

impl From<String> for EntryHash {
    fn from(s: String) -> EntryHash {
        EntryHash::from(s.as_str())
    }
}

impl From<&String> for EntryHash {
    fn from(s: &String) -> EntryHash {
        (*s).to_owned().into()
    }
}

impl From<EntryHash> for String {
    fn from(h: EntryHash) -> String {
        h.hash_string().to_owned().into()
    }
}

impl From<&EntryHash> for String {
    fn from(h: &EntryHash) -> String {
        (*h).to_owned().into()
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

impl From<&HashString> for AspectHash {
    fn from(s: &HashString) -> AspectHash {
        (*s).to_owned().into()
    }
}

impl From<AspectHash> for HashString {
    fn from(h: AspectHash) -> HashString {
        h.0
    }
}

impl From<&AspectHash> for HashString {
    fn from(h: &AspectHash) -> HashString {
        (*h).to_owned().into()
    }
}

impl From<&str> for AspectHash {
    fn from(s: &str) -> AspectHash {
        HashString::from(s).into()
    }
}

impl From<String> for AspectHash {
    fn from(s: String) -> AspectHash {
        AspectHash::from(s.as_str())
    }
}

impl From<&String> for AspectHash {
    fn from(s: &String) -> AspectHash {
        (*s).to_owned().into()
    }
}

impl From<AspectHash> for String {
    fn from(s: AspectHash) -> String {
        s.hash_string().to_owned().into()
    }
}

impl From<&AspectHash> for String {
    fn from(s: &AspectHash) -> String {
        (*s).to_owned().into()
    }
}

impl HashStringNewType for AspectHash {
    fn hash_string(&self) -> &HashString {
        &self.0
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

impl From<&HashString> for NetworkHash {
    fn from(s: &HashString) -> NetworkHash {
        (*s).to_owned().into()
    }
}

impl From<NetworkHash> for HashString {
    fn from(h: NetworkHash) -> HashString {
        h.0
    }
}

impl From<&NetworkHash> for HashString {
    fn from(h: &NetworkHash) -> HashString {
        (*h).to_owned().into()
    }
}

impl<'a> From<&'a str> for NetworkHash {
    fn from(s: &str) -> NetworkHash {
        HashString::from(s).into()
    }
}

impl From<String> for NetworkHash {
    fn from(s: String) -> NetworkHash {
        NetworkHash::from(s.as_str())
    }
}

impl From<&String> for NetworkHash {
    fn from(s: &String) -> NetworkHash {
        (*s).to_owned().into()
    }
}

impl From<NetworkHash> for String {
    fn from(s: NetworkHash) -> String {
        s.hash_string().to_owned().into()
    }
}

impl From<&NetworkHash> for String {
    fn from(s: &NetworkHash) -> String {
        (*s).to_owned().into()
    }
}

impl HashStringNewType for NetworkHash {
    fn hash_string(&self) -> &HashString {
        &self.0
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

impl From<&HashString> for AgentPubKey {
    fn from(s: &HashString) -> AgentPubKey {
        (*s).to_owned().into()
    }
}

impl From<AgentPubKey> for HashString {
    fn from(h: AgentPubKey) -> HashString {
        h.0
    }
}

impl From<&AgentPubKey> for HashString {
    fn from(h: &AgentPubKey) -> HashString {
        (*h).to_owned().into()
    }
}

impl From<&str> for AgentPubKey {
    fn from(s: &str) -> AgentPubKey {
        HashString::from(s).into()
    }
}

impl From<String> for AgentPubKey {
    fn from(s: String) -> AgentPubKey {
        HashString::from(s).into()
    }
}

impl From<&String> for AgentPubKey {
    fn from(s: &String) -> AgentPubKey {
        (*s).to_owned().into()
    }
}

impl From<AgentPubKey> for String {
    fn from(s: AgentPubKey) -> String {
        s.hash_string().to_owned().into()
    }
}

impl From<&AgentPubKey> for String {
    fn from(s: &AgentPubKey) -> String {
        (*s).to_owned().into()
    }
}

impl HashStringNewType for AgentPubKey {
    fn hash_string(&self) -> &HashString {
        &self.0
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

impl From<&HashString> for NodePubKey {
    fn from(s: &HashString) -> NodePubKey {
        (*s).to_owned().into()
    }
}

impl From<NodePubKey> for HashString {
    fn from(h: NodePubKey) -> HashString {
        h.0
    }
}

impl From<&NodePubKey> for HashString {
    fn from(h: &NodePubKey) -> HashString {
        (*h).to_owned().into()
    }
}

impl From<&str> for NodePubKey {
    fn from(s: &str) -> NodePubKey {
        HashString::from(s).into()
    }
}

impl From<String> for NodePubKey {
    fn from(s: String) -> NodePubKey {
        NodePubKey::from(s.as_str())
    }
}

impl From<&String> for NodePubKey {
    fn from(s: &String) -> NodePubKey {
        (*s).to_owned().into()
    }
}

impl From<NodePubKey> for String {
    fn from(s: NodePubKey) -> String {
        s.hash_string().to_owned().into()
    }
}

impl From<&NodePubKey> for String {
    fn from(s: &NodePubKey) -> String {
        (*s).to_owned().into()
    }
}

impl HashStringNewType for NodePubKey {
    fn hash_string(&self) -> &HashString {
        &self.0
    }
}

#[cfg(test)]
pub mod tests {

    use super::{AspectHash, SpaceHash};
    use crate::{
        fixture::space_hash_fresh,
        types::{AgentPubKey, EntryHash, HashStringNewType, NetworkHash, NodePubKey},
    };
    use holochain_persistence_api::{fixture::test_hash_a, hash::HashString};
    use uuid::Uuid;

    fn display_for_t<T: std::fmt::Display + From<HashString>>() {
        let s = Uuid::new_v4().to_string();
        let t = T::from(HashString::from(s.clone()));

        assert_eq!(s, format!("{}", &t),);
    }

    fn t_from_hash_string<T: HashStringNewType + Clone + From<HashString>>() {
        let hash = test_hash_a();

        // cloned
        let t = T::from(hash.clone());
        assert_eq!(&hash, t.hash_string());
    }

    fn t_from_hash_string_ref<'a, T: From<&'a HashString>>() {
        // i don't know how to test this
        // some lifetime weirdness...
    }

    fn hash_string_from_t<T: Into<HashString> + Clone + HashStringNewType>(t: T) {
        // cloned
        let hash_string: HashString = t.clone().into();

        assert_eq!(t.hash_string(), &hash_string,);
    }

    fn t_from_str<'a, T: HashStringNewType + From<&'a str> + From<String> + From<&'a String>>() {
        let str = "foo";

        let t = T::from(str);

        assert_eq!(t.hash_string(), &HashString::from(str),);

        let string = String::from(str);

        // cloned string
        let t = T::from(string.clone());

        assert_eq!(String::from(t.hash_string().clone()), string,);

        // reference
        // TODO fix lifetimes
        // let t = T::from(&string);
        // assert_eq!(String::from(t.hash_string().clone()), string,);
    }

    #[test]
    fn test_space_hash() {
        display_for_t::<SpaceHash>();
        t_from_hash_string::<SpaceHash>();
        t_from_hash_string_ref::<SpaceHash>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&SpaceHash::default());

        t_from_str::<SpaceHash>();
        let _ = String::from(&SpaceHash::default());
        let _ = String::from(SpaceHash::from("foo"));
    }

    #[test]
    fn test_entry_hash() {
        display_for_t::<EntryHash>();
        t_from_hash_string::<EntryHash>();
        t_from_hash_string_ref::<EntryHash>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&EntryHash::default());

        t_from_str::<EntryHash>();
        let _ = String::from(&EntryHash::default());
        let _ = String::from(EntryHash::from("foo"));
    }

    #[test]
    fn test_aspect_hash() {
        display_for_t::<AspectHash>();
        t_from_hash_string::<AspectHash>();
        t_from_hash_string_ref::<AspectHash>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&AspectHash::default());

        t_from_str::<AspectHash>();
        let _ = String::from(&AspectHash::default());
        let _ = String::from(AspectHash::from("foo"));
    }

    #[test]
    fn test_network_hash() {
        display_for_t::<NetworkHash>();
        t_from_hash_string::<NetworkHash>();
        t_from_hash_string_ref::<NetworkHash>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&NetworkHash::default());

        t_from_str::<NetworkHash>();
        let _ = String::from(&NetworkHash::default());
        let _ = String::from(NetworkHash::from("foo"));
    }

    #[test]
    fn test_agent_pub_key() {
        display_for_t::<AgentPubKey>();
        t_from_hash_string::<AgentPubKey>();
        t_from_hash_string_ref::<AgentPubKey>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&AgentPubKey::default());

        t_from_str::<AgentPubKey>();
        let _ = String::from(&AgentPubKey::default());
        let _ = String::from(AgentPubKey::from("foo"));
    }

    #[test]
    fn test_node_pub_key() {
        display_for_t::<NodePubKey>();
        t_from_hash_string::<NodePubKey>();
        t_from_hash_string_ref::<NodePubKey>();

        hash_string_from_t(space_hash_fresh());
        let _ = HashString::from(&NodePubKey::default());

        t_from_str::<NodePubKey>();
        let _ = String::from(&NodePubKey::default());
        let _ = String::from(NodePubKey::from("foo"));
    }
}
