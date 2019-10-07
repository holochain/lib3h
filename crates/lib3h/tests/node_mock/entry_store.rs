use lib3h_protocol::{
    data_types::{EntryAspectData, EntryData},
    types::*,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct EntryStore {
    pub store: HashMap<EntryHash, HashMap<AspectHash, EntryAspectData>>,
}

impl EntryStore {
    pub fn new() -> Self {
        EntryStore {
            store: HashMap::new(),
        }
    }

    /// Check if this value is already stored
    #[allow(dead_code)]
    pub fn has(&self, entry_address: &EntryHash, aspect_address: &AspectHash) -> bool {
        let maybe_map = self.store.get(entry_address);
        if maybe_map.is_none() {
            return false;
        }
        maybe_map.unwrap().get(aspect_address).is_some()
    }

    ///
    pub fn insert_entry(&mut self, entry: &EntryData) {
        trace!("EntryStore: adding content for '{:?}'", entry.entry_address);
        if self.store.get(&entry.entry_address).is_none() {
            let mut map = HashMap::new();
            trace!("  -> first content!");
            for aspect in entry.aspect_list.clone() {
                map.insert(aspect.aspect_address.clone(), aspect.clone());
            }
            self.store.insert(entry.entry_address.clone(), map);
            return;
        }
        if let Some(map) = self.store.get_mut(&entry.entry_address) {
            for aspect in entry.aspect_list.clone() {
                map.insert(aspect.aspect_address.clone(), aspect.clone());
            }
        }
    }

    ///
    pub fn insert_aspect(&mut self, entry_address: &EntryHash, aspect: &EntryAspectData) {
        trace!(
            "EntryStore: adding content for '{:?}': {:?}",
            entry_address,
            aspect.aspect_address,
        );
        if self.store.get(entry_address).is_none() {
            let mut map = HashMap::new();
            trace!("  -> first content!");
            map.insert(aspect.aspect_address.clone(), aspect.clone());
            self.store.insert(entry_address.clone(), map);
            return;
        }
        if let Some(map) = self.store.get_mut(entry_address) {
            map.insert(aspect.aspect_address.clone(), aspect.clone());
        }
    }

    /// Get all values for a meta_key as a vec
    pub fn get(&self, entry_address: &EntryHash) -> Option<EntryData> {
        let aspect_map = self.store.get(entry_address)?;
        let aspect_list: Vec<EntryAspectData> = aspect_map.iter().map(|(_, v)| v.clone()).collect();
        return if aspect_list.is_empty() {
            None
        } else {
            Some(EntryData {
                entry_address: entry_address.clone(),
                aspect_list,
            })
        };
    }

    /// Get all values for a meta_key as a vec
    #[allow(dead_code)]
    pub fn get_aspect(
        &self,
        entry_address: &EntryHash,
        aspect_address: &AspectHash,
    ) -> Option<EntryAspectData> {
        let maybe_entry = self.get(entry_address);
        if maybe_entry.is_none() {
            return None;
        }
        return maybe_entry.unwrap().get(aspect_address);
    }
}
