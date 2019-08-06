use std::collections::HashMap;

pub type TrackId = String;
pub type TrackIdRef = str;

struct TrackItem<T> {
    pub value: Option<T>,
    pub expires_ms: u64,
}

pub struct Tracker<T> {
    id_prefix: String,
    timeout_ms: u64,
    map: HashMap<TrackId, TrackItem<T>>,
}

impl<T> Tracker<T> {
    pub fn new(id_prefix: &str, timeout_ms: u64) -> Self {
        Self {
            id_prefix: id_prefix.to_string(),
            timeout_ms,
            map: HashMap::new(),
        }
    }

    pub fn has(&self, id: &TrackIdRef) -> bool {
        self.map.contains_key(id)
    }

    pub fn get(&self, id: &TrackIdRef) -> Option<&T> {
        match self.map.get(id) {
            Some(item) => item.value.as_ref(),
            None => None,
        }
    }

    pub fn gen_id(&self) -> TrackId {
        format!("{}{}", self.id_prefix, nanoid::simple())
    }

    pub fn reserve(&mut self) -> TrackId {
        let id = self.gen_id();

        self.map.insert(id.clone(), self.priv_new_track_item(None));

        id
    }

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

    pub fn remove(&mut self, id: &TrackIdRef) -> Option<T> {
        match self.map.remove(id) {
            Some(s) => s.value,
            None => None,
        }
    }

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
