use crate::simchat::{MessageList, SimChatMessage};
use lib3h_protocol::Address;
use std::collections::HashMap;
pub struct Store(HashMap<Address, HashMap<Address, HashMap<Address, SimChatMessage>>>); // space_address -> anchor_addres -> message_address

impl Store {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // pub fn get(
    //     &self,
    //     space_address: &Address,
    //     base_address: &Address,
    //     message_address: &Address,
    // ) -> Option<&SimChatMessage> {
    //     self.0
    //         .get(&space_address)?
    //         .get(&base_address)?
    //         .get(&message_address)
    // }

    pub fn get_all_messages(
        &self,
        space_address: &Address,
        base_address: &Address,
    ) -> Option<MessageList> {
        Some(MessageList(
            self.0
                .get(&space_address)?
                .get(&base_address)?
                .values()
                .map(|s| s.clone())
                .collect(),
        ))
    }

    pub fn insert(
        &mut self,
        space_address: &Address,
        base_address: &Address,
        message_address: &Address,
        message: SimChatMessage,
    ) {
        let mut space = self
            .0
            .get(space_address)
            .map(|hm| hm.clone())
            .unwrap_or_default();
        let mut base = space
            .get(base_address)
            .map(|hm| hm.clone())
            .unwrap_or_default();

        base.insert(message_address.clone(), message);
        space.insert(base_address.clone(), base.clone());
        self.0.insert(space_address.clone(), space.clone());
    }
}

pub struct StoreEntryList(HashMap<Address, HashMap<Address, Vec<Address>>>);

impl StoreEntryList {
    pub fn new() -> Self {
        StoreEntryList(HashMap::new())
    }

    pub fn get(&self, space_address: &Address) -> Option<&HashMap<Address, Vec<Address>>> {
        self.0.get(space_address)
    }

    pub fn insert(
        &mut self,
        space_address: &Address,
        entry_address: &Address,
        aspect_address: &Address,
    ) {
        let mut entry_map = self
            .0
            .get(space_address)
            .map(|hm| hm.clone())
            .unwrap_or_default();
        let mut aspect_list = entry_map
            .get(entry_address)
            .map(|hm| hm.clone())
            .unwrap_or_default();
        aspect_list.push(aspect_address.clone());
        entry_map.insert(entry_address.clone(), aspect_list);
        self.0.insert(space_address.clone(), entry_map.clone());
    }
}
