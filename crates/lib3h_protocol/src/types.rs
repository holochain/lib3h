use holochain_persistence_api::hash::HashString;
use std::{convert::TryInto, fmt};

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

//impl<'a> From<&Vec<u8>> for SpaceHash {
//    fn from(v: &Vec<u8>) -> SpaceHash {
//        HashString::from(v.clone())
//    }
//}
//
//impl From<Vec<u8>> for SpaceHash {
//    fn from(v: Vec<u8>) -> SpaceHash {
//        HashString::from(v.to_base58())
//    }
//}
//
//impl TryInto<Vec<u8>> for SpaceHash {
//    type Error = rust_base58::base58::FromBase58Error;
//    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
//        self.0.from_base58()
//    }
//}
//
//impl<'a> TryInto<Vec<u8>> for &'a SpaceHash {
//    type Error = rust_base58::base58::FromBase58Error;
//    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
//        self.clone().try_into()
//    }
//}

impl SpaceHash {
    pub fn new() -> SpaceHash {
        SpaceHash(HashString::new())
    }
}

//--------------------------------------------------------------------------------------------------
// EntryHash
//--------------------------------------------------------------------------------------------------
