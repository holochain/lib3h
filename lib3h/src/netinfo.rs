use chrono;
use ident;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub addr: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeInfo {
    pub id: Vec<u8>,
    pub u32_tag: u32,
    pub pub_keys: ident::BundlePubIdentity,
    pub endpoint: Endpoint,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub connections: Vec<Vec<u8>>,
    pub metadata: HashMap<String, Vec<u8>>,
}

impl NodeInfo {
    pub fn new() -> Self {
        NodeInfo {
            id: Vec::new(),
            u32_tag: 0,
            pub_keys: ident::BundlePubIdentity {
                pub_keys: Vec::new(),
            },
            endpoint: Endpoint {
                addr: String::from(""),
                port: 0,
            },
            last_seen: chrono::Utc::now(),
            connections: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn update_if_newer(&mut self, oth: &NodeInfo) {
        if (oth.last_seen < self.last_seen) {
            return;
        }

        *self = oth.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_update_if_newer() {
        let mut one = NodeInfo::new();
        one.last_seen = chrono::Utc::now() - chrono::Duration::days(1);
        one.u32_tag = 1;

        let mut two = NodeInfo::new();
        two.u32_tag = 2;

        one.update_if_newer(&two);
        two.u32_tag = 0;

        assert_eq!(one.u32_tag, 2);
    }

    #[test]
    fn it_should_not_update_if_older() {
        let mut one = NodeInfo::new();
        one.last_seen = chrono::Utc::now() + chrono::Duration::days(1);
        one.u32_tag = 1;

        let mut two = NodeInfo::new();
        two.u32_tag = 2;

        one.update_if_newer(&two);
        two.u32_tag = 0;

        assert_eq!(one.u32_tag, 1);
    }
}
