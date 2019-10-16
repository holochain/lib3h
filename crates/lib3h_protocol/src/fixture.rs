use crate::types::SpaceHash;
use holochain_persistence_api::fixture::test_hash_a;

pub fn space_hash_fresh() -> SpaceHash {
    SpaceHash::from(test_hash_a())
}
