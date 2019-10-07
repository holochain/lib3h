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
