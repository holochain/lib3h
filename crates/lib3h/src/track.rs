use std::collections::HashMap;

/// request_id type
pub type TrackId = String;
/// request_id ref type
pub type TrackIdRef = str;

/// internal - data needed to track this async action
struct TrackItem<T> {
    /// userdata needed to keep track of this request
    pub value: Option<T>,
    /// when this request should expire / timeout - set as now + timeout
    pub expires_ms: u64,
}

/// Helper to keep track of request_ids for message correlation with core or
/// other p2p nodes
pub struct Tracker<T> {
    id_prefix: String,
    timeout_ms: u64,
    map: HashMap<TrackId, TrackItem<T>>,
}

impl<T> Tracker<T> {
    /// create a new tracker instance
    /// ids will be prefixed with id_prefix
    /// request_ids will timeout after timeout_ms
    pub fn new(id_prefix: &str, timeout_ms: u64) -> Self {
        Self {
            id_prefix: id_prefix.to_string(),
            timeout_ms,
            map: HashMap::new(),
        }
    }

    /// `true` if we are still tracking `id`
    pub fn has(&self, id: &TrackIdRef) -> bool {
        self.map.contains_key(id)
    }

    /// if we are tracking `id` return a reference to the user data
    pub fn get(&self, id: &TrackIdRef) -> Option<&T> {
        match self.map.get(id) {
            Some(item) => item.value.as_ref(),
            None => None,
        }
    }

    /// generate a request_id for this tracker
    pub fn gen_id(&self) -> TrackId {
        format!("{}{}", self.id_prefix, nanoid::simple())
    }

    /// reserve a space in the tracker for a new request_id
    /// this will start counting down on the timeout
    pub fn reserve(&mut self) -> TrackId {
        let id = self.gen_id();

        self.map.insert(id.clone(), self.priv_new_track_item(None));

        id
    }

    /// set userdata for `id`, will return any previous userdata at that id
    /// if we are not tracking anything for an id, will start a new tracker
    pub fn set(&mut self, id: &TrackIdRef, value: Option<T>) -> Option<T> {
        match self.map.get_mut(id) {
            Some(item) => std::mem::replace(&mut item.value, value),
            None => {
                self.map
                    .insert(id.to_string(), self.priv_new_track_item(value));
                None
            }
        }
    }

    /// stop tracking an id, returning the user data
    pub fn remove(&mut self, id: &TrackIdRef) -> Option<T> {
        match self.map.remove(id) {
            Some(s) => s.value,
            None => None,
        }
    }

    /// process our tracking ids, and return all those that have timed out
    /// Remove any ids which have timed out from the tracker
    pub fn process_timeouts(&mut self) -> Vec<(TrackId, Option<T>)> {
        let mut out = Vec::new();

        let now = crate::time::since_epoch_ms();

        let expire_list: Vec<String> = self
            .map
            .iter()
            .filter_map(|(k, v)| {
                if v.expires_ms > now {
                    return None;
                }
                Some(k.clone())
            })
            .collect();

        for id in expire_list {
            let value = self.remove(&id);
            out.push((id, value))
        }

        out
    }

    // -- private -- //

    /// helper for creating internal TrackItem instances
    fn priv_new_track_item(&self, value: Option<T>) -> TrackItem<T> {
        TrackItem {
            value,
            expires_ms: crate::time::since_epoch_ms() + self.timeout_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_track() {
        let mut t: Tracker<String> = Tracker::new("test1_", 1000);
        assert!(!t.has("aoeu"));
        let id = t.reserve();
        assert!(t.has(&id));
        assert_eq!(None, t.set(&id, Some("test_val".to_string())));
        assert_eq!(Some("test_val".to_string()), t.set(&id, None));
        assert_eq!(None, t.set(&id, Some("test_val2".to_string())));
        assert_eq!(Some(&"test_val2".to_string()), t.get(&id));
        assert_eq!(Some("test_val2".to_string()), t.remove(&id));
        assert!(!t.has(&id));
    }

    #[test]
    pub fn it_should_timeout() {
        let mut t: Tracker<String> = Tracker::new("test2_", 1);

        let id1 = t.reserve();
        t.set(&id1, Some("test_a".to_string()));

        let id2 = t.reserve();
        t.set(&id2, Some("test_b".to_string()));

        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = t.process_timeouts();

        assert_eq!(2, result.len());
        for (id, value) in result {
            assert!(id == id1 || id == id2);
            let value = value.unwrap();
            assert!(&value == "test_a" || &value == "test_b");
        }
    }
}
