use holochain_persistence_api::hash::HashString;
use std::fmt;

//--------------------------------------------------------------------------------------------------
// SpaceHash: newtype for HashString
//--------------------------------------------------------------------------------------------------

#[derive(PartialOrd, PartialEq, Eq, Ord, Clone, Debug, Serialize, Deserialize, Default, Hash)]
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
