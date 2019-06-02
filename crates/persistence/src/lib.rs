//! Holochain Persistence Api
//! This crate defines apis for a content addressable storage (cas) and a generic 
//! entity attribute value index (eavi).
#![feature(try_from)]
#![feature(try_trait)]
#![feature(never_type)]
#![warn(unused_extern_crates)]

//extern crate base64;
extern crate chrono;
extern crate futures;
extern crate multihash;
extern crate rust_base58;
extern crate serde;
//#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lib3h_persistence_derive;

extern crate uuid;

pub mod cas;
pub mod eav;
pub mod error;
#[macro_use]
extern crate objekt;
pub mod hash;
pub mod json;

pub const GIT_HASH: &str = env!(
    "GIT_HASH",
    "failed to obtain git hash from build environment. Check build.rs"
);

// not docker build friendly
// https://circleci.com/gh/holochain/holochain-rust/10757
#[cfg(feature = "broken-tests")]
#[cfg(test)]
mod test_hash {
    use super::*;

    #[test]
    fn test_hash() {
        assert_eq!(GIT_HASH.chars().count(), 40);
        assert!(
            GIT_HASH.is_ascii(),
            "GIT HASH contains non-ascii characters"
        );
    }
}
