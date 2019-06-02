use lib3h_persistence::{
    eav::{Attribute, EaviQuery, EntityAttributeValueIndex, EntityAttributeValueStorage},
    error::{PersistenceError, PersistenceResult},
};
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use std::{
    collections::BTreeSet,
    fmt::{Debug, Error, Formatter},
    marker::{PhantomData, Send, Sync},
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};
use uuid::Uuid;

const PERSISTENCE_INTERVAL: Duration = Duration::from_millis(5000);

#[derive(Clone)]
pub struct EavPickleStorage<A: Attribute> {
    db: Arc<RwLock<PickleDb>>,
    id: Uuid,
    attribute: PhantomData<A>,
}

impl<A: Attribute> EavPickleStorage<A> {
    pub fn new<P: AsRef<Path> + Clone>(db_path: P) -> EavPickleStorage<A> {
        let eav_db = db_path.as_ref().join("eav").with_extension("db");
        EavPickleStorage {
            id: Uuid::new_v4(),
            db: Arc::new(RwLock::new(
                PickleDb::load(
                    eav_db.clone(),
                    PickleDbDumpPolicy::PeriodicDump(PERSISTENCE_INTERVAL),
                    SerializationMethod::Cbor,
                )
                .unwrap_or_else(|_| {
                    PickleDb::new(
                        eav_db,
                        PickleDbDumpPolicy::PeriodicDump(PERSISTENCE_INTERVAL),
                        SerializationMethod::Cbor,
                    )
                }),
            )),
            attribute: PhantomData,
        }
    }
}

impl<A: Attribute> Debug for EavPickleStorage<A> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("EavPickleStorage")
            .field("id", &self.id)
            .finish()
    }
}

impl<A: Attribute> EntityAttributeValueStorage<A> for EavPickleStorage<A>
where
    A: Sync + Send + serde::de::DeserializeOwned,
{
    fn add_eavi(
        &mut self,
        eav: &EntityAttributeValueIndex<A>,
    ) -> PersistenceResult<Option<EntityAttributeValueIndex<A>>> {
        let mut inner = self.db.write().unwrap();

        //hate to introduce mutability but it is saved by the immutable clones at the end
        let mut index_str = eav.index().to_string();
        let mut value = inner.get::<EntityAttributeValueIndex<A>>(&index_str);
        let mut new_eav = eav.clone();
        while value.is_some() {
            new_eav =
                EntityAttributeValueIndex::new(&eav.entity(), &eav.attribute(), &eav.value())?;
            index_str = new_eav.index().to_string();
            value = inner.get::<EntityAttributeValueIndex<A>>(&index_str);
        }
        inner
            .set(&*index_str, &new_eav)
            .map_err(|e| PersistenceError::ErrorGeneric(e.to_string()))?;
        Ok(Some(new_eav.clone()))
    }

    fn fetch_eavi(
        &self,
        query: &EaviQuery<A>,
    ) -> Result<BTreeSet<EntityAttributeValueIndex<A>>, PersistenceError> {
        let inner = self.db.read()?;

        //this not too bad because it is lazy evaluated
        let entries = inner
            .iter()
            .map(|item| item.get_value())
            .filter(|filter| filter.is_some())
            .map(|y| y.unwrap())
            .collect::<BTreeSet<EntityAttributeValueIndex<A>>>();
        let entries_iter = entries.iter().cloned();
        Ok(query.run(entries_iter))
    }
}

#[cfg(test)]
pub mod tests {
    use crate::eav::pickle::EavPickleStorage;
    use lib3h_persistence::{
        cas::{
            content::{AddressableContent, ExampleAddressableContent},
            storage::EavTestSuite,
        },
        eav::ExampleAttribute,
        json::RawString,
    };
    use tempfile::tempdir;

    #[test]
    fn pickle_eav_round_trip() {
        let temp = tempdir().expect("test was supposed to create temp dir");

        let temp_path = String::from(temp.path().to_str().expect("temp dir could not be string"));
        let entity_content =
            ExampleAddressableContent::try_from_content(&RawString::from("foo").into()).unwrap();
        let attribute = ExampleAttribute::WithPayload("favourite-color".to_string());
        let value_content =
            ExampleAddressableContent::try_from_content(&RawString::from("blue").into()).unwrap();

        EavTestSuite::test_round_trip(
            EavPickleStorage::new(temp_path),
            entity_content,
            attribute,
            value_content,
        )
    }

    #[test]
    fn pickle_eav_one_to_many() {
        let temp = tempdir().expect("test was supposed to create temp dir");
        let temp_path = String::from(temp.path().to_str().expect("temp dir could not be string"));
        let eav_storage = EavPickleStorage::new(temp_path);
        EavTestSuite::test_one_to_many::<
            ExampleAddressableContent,
            EavPickleStorage<ExampleAttribute>,
        >(eav_storage);
    }

    #[test]
    fn pickle_eav_many_to_one() {
        let temp = tempdir().expect("test was supposed to create temp dir");
        let temp_path = String::from(temp.path().to_str().expect("temp dir could not be string"));
        let eav_storage = EavPickleStorage::new(temp_path);
        EavTestSuite::test_many_to_one::<
            ExampleAddressableContent,
            EavPickleStorage<ExampleAttribute>,
        >(eav_storage);
    }

    #[test]
    fn pickle_eav_range() {
        let temp = tempdir().expect("test was supposed to create temp dir");
        let temp_path = String::from(temp.path().to_str().expect("temp dir could not be string"));
        let eav_storage = EavPickleStorage::new(temp_path);
        EavTestSuite::test_range::<ExampleAddressableContent, EavPickleStorage<ExampleAttribute>>(
            eav_storage,
        );
    }

    #[test]
    fn pickle_eav_prefixes() {
        let temp = tempdir().expect("test was supposed to create temp dir");
        let temp_path = String::from(temp.path().to_str().expect("temp dir could not be string"));
        let eav_storage = EavPickleStorage::new(temp_path);
        EavTestSuite::test_multiple_attributes::<
            ExampleAddressableContent,
            EavPickleStorage<ExampleAttribute>,
        >(
            eav_storage,
            vec!["a_", "b_", "c_", "d_"]
                .into_iter()
                .map(|p| ExampleAttribute::WithPayload(p.to_string() + "one_to_many"))
                .collect(),
        );
    }

}
