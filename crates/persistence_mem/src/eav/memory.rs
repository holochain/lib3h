use persistence_api::{
    eav::{
        increment_key_till_no_collision, Attribute, EaviQuery, EntityAttributeValueIndex,
        EntityAttributeValueStorage,
    },
};
use json_api::error::JsonResult;
use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
};

use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct EavMemoryStorage<A: Attribute> {
    storage: Arc<RwLock<BTreeSet<EntityAttributeValueIndex<A>>>>,
    id: Uuid,
}

impl<A: Attribute> PartialEq for EavMemoryStorage<A> {
    fn eq(&self, other: &EavMemoryStorage<A>) -> bool {
        self.id == other.id
    }
}

impl<A: Attribute> Default for EavMemoryStorage<A> {
    fn default() -> EavMemoryStorage<A> {
        EavMemoryStorage {
            storage: Arc::new(RwLock::new(BTreeSet::new())),
            id: Uuid::new_v4(),
        }
    }
}

impl<A: Attribute> EavMemoryStorage<A> {
    pub fn new() -> EavMemoryStorage<A> {
        Default::default()
    }
}

impl<A: Attribute> EntityAttributeValueStorage<A> for EavMemoryStorage<A>
where
    A: Send + Sync,
{
    fn add_eavi(
        &mut self,
        eav: &EntityAttributeValueIndex<A>,
    ) -> JsonResult<Option<EntityAttributeValueIndex<A>>> {
        let mut map = self.storage.write()?;
        let new_eav = increment_key_till_no_collision(eav.clone(), map.clone())?;
        map.insert(new_eav.clone());
        Ok(Some(new_eav.clone()))
    }

    fn fetch_eavi(
        &self,
        query: &EaviQuery<A>,
    ) -> JsonResult<BTreeSet<EntityAttributeValueIndex<A>>> {
        let map = self.storage.read()?;
        let iter = map.iter().cloned();
        Ok(query.run(iter))
    }
}

#[cfg(test)]
pub mod tests {
    use crate::eav::memory::EavMemoryStorage;
    use persistence_api::{
        cas::{
            content::{AddressableContent, ExampleAddressableContent},
            storage::EavTestSuite,
        },
        eav::ExampleAttribute,
        json::RawString,
    };

    #[test]
    fn memory_eav_round_trip() {
        let entity_content =
            ExampleAddressableContent::try_from_content(&RawString::from("foo").into()).unwrap();
        let attribute = ExampleAttribute::WithPayload("favourite-color".to_string());
        let value_content =
            ExampleAddressableContent::try_from_content(&RawString::from("blue").into()).unwrap();
        EavTestSuite::test_round_trip(
            EavMemoryStorage::new(),
            entity_content,
            attribute,
            value_content,
        )
    }

    #[test]
    fn memory_eav_one_to_many() {
        let eav_storage = EavMemoryStorage::new();
        EavTestSuite::test_one_to_many::<
            ExampleAddressableContent,
            ExampleAttribute,
            EavMemoryStorage<ExampleAttribute>,
        >(eav_storage, &ExampleAttribute::default())
    }

    #[test]
    fn memory_eav_many_to_one() {
        let eav_storage = EavMemoryStorage::new();
        EavTestSuite::test_many_to_one::<
            ExampleAddressableContent,
            ExampleAttribute,
            EavMemoryStorage<ExampleAttribute>,
        >(eav_storage, &ExampleAttribute::default())
    }

    #[test]
    fn example_eav_range() {
        let eav_storage = EavMemoryStorage::new();
        EavTestSuite::test_range::<
            ExampleAddressableContent,
            ExampleAttribute,
            EavMemoryStorage<ExampleAttribute>,
        >(eav_storage, &ExampleAttribute::default());
    }

    #[test]
    fn file_eav_prefixes() {
        let eav_storage = EavMemoryStorage::new();
        EavTestSuite::test_multiple_attributes::<
            ExampleAddressableContent,
            ExampleAttribute,
            EavMemoryStorage<ExampleAttribute>,
        >(
            eav_storage,
            vec!["a_", "b_", "c_", "d_"]
                .into_iter()
                .map(|p| ExampleAttribute::WithPayload(p.to_string() + "one_to_many"))
                .collect(),
        );
    }

}
