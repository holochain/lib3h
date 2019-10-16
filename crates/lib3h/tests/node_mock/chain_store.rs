use super::entry_store::EntryStore;
use lib3h_protocol::{
    data_types::{EntryAspectData, EntryData},
    types::*,
    Address,
};
use std::collections::HashMap;

/// Holds Space-specific data
#[derive(Clone)]
pub struct ChainStore {
    space_address: SpaceHash,
    stored_entry_store: EntryStore,
    authored_entry_store: EntryStore,
}

impl ChainStore {
    pub fn new(space_address: &SpaceHash) -> Self {
        ChainStore {
            space_address: space_address.clone(),
            stored_entry_store: EntryStore::new(),
            authored_entry_store: EntryStore::new(),
        }
    }

    pub fn get_entry(&self, entry_address: &Address) -> Option<EntryData> {
        let mut has_aspects = false;
        let mut entry = EntryData {
            entry_address: entry_address.clone(),
            aspect_list: vec![],
        };
        // Append what we have in `authored_entry_store`
        let maybe_entry_store = self.authored_entry_store.get(&entry_address);
        if let Some(mut local_entry) = maybe_entry_store {
            // println!("ChainStore.get_entry: authored_entry_store has {}", local_entry.aspect_list.len());
            entry.aspect_list.append(&mut local_entry.aspect_list);
            has_aspects = true;
        }
        // Append what we have in `stored_entry_store`
        let maybe_entry_store = self.stored_entry_store.get(&entry_address);
        if let Some(mut local_entry) = maybe_entry_store {
            // println!("ChainStore.get_entry: stored_entry_store has {}", local_entry.aspect_list.len());
            entry.aspect_list.append(&mut local_entry.aspect_list);
            has_aspects = true;
        }
        // Done
        return if has_aspects { Some(entry) } else { None };
    }

    // -- insert -- //

    /// Return Err if Entry is already known
    pub fn author_entry(&mut self, entry: &EntryData) -> Result<(), ()> {
        if self.has(&entry.entry_address) {
            return Err(());
        }
        self.authored_entry_store.insert_entry(entry);
        Ok(())
    }

    /// Return Err if Entry is already known
    pub fn hold_entry(&mut self, entry: &EntryData) -> Result<(), ()> {
        if self.has(&entry.entry_address) {
            return Err(());
        }
        self.stored_entry_store.insert_entry(entry);
        Ok(())
    }

    /// Return Err if Aspect is already known
    pub fn author_aspect(
        &mut self,
        entry_address: &Address,
        aspect: &EntryAspectData,
    ) -> Result<(), ()> {
        if self
            .get_aspect(entry_address, &aspect.aspect_address)
            .is_some()
        {
            return Err(());
        }
        self.authored_entry_store
            .insert_aspect(entry_address, aspect);
        Ok(())
    }

    /// Return Err if Aspect is already known
    pub fn hold_aspect(
        &mut self,
        entry_address: &Address,
        aspect: &EntryAspectData,
    ) -> Result<(), ()> {
        if self
            .get_aspect(entry_address, &aspect.aspect_address)
            .is_some()
        {
            return Err(());
        }
        self.stored_entry_store.insert_aspect(entry_address, aspect);
        Ok(())
    }

    // -- has -- //

    pub fn has_authored(&self, entry_address: &Address) -> bool {
        self.authored_entry_store.get(&entry_address).is_some()
    }

    pub fn has_stored(&self, entry_address: &Address) -> bool {
        self.stored_entry_store.get(&entry_address).is_some()
    }

    pub fn has(&self, entry_address: &Address) -> bool {
        self.has_authored(entry_address) || self.has_stored(entry_address)
    }

    pub fn get_aspect(
        &self,
        entry_address: &Address,
        aspect_address: &Address,
    ) -> Option<EntryAspectData> {
        let maybe_entry = self.get_entry(entry_address);
        if let Some(entry) = maybe_entry {
            return entry.get(aspect_address);
        }
        None
    }

    // -- Getters -- //

    pub fn get_authored_store(&self) -> HashMap<Address, HashMap<Address, EntryAspectData>> {
        self.authored_entry_store.store.clone()
    }

    pub fn get_stored_store(&self) -> HashMap<Address, HashMap<Address, EntryAspectData>> {
        self.stored_entry_store.store.clone()
    }

    #[allow(dead_code)]
    pub fn space_address(&self) -> SpaceHash {
        self.space_address.clone()
    }
}
