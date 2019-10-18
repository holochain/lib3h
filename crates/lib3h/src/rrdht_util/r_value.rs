use crate::rrdht_util::{Arc, ARC_LENGTH_MAX};

/// builder for our knowledge of a single agent's storage / reachability
pub struct RValuePeerRecord {
    storage_arc: Arc,
    uptime_0_to_1: f64,
}

impl Default for RValuePeerRecord {
    fn default() -> Self {
        Self {
            storage_arc: Arc::new(0.into(), 0),
            uptime_0_to_1: 0.0,
        }
    }
}

impl RValuePeerRecord {
    /// what arc is this agent claiming to store
    pub fn storage_arc(mut self, storage_arc: Arc) -> Self {
        self.storage_arc = storage_arc;
        self
    }

    /// what is this agent's uptime
    /// - 0 = never reachable
    /// - 1 = always reachable
    pub fn uptime_0_to_1(mut self, uptime_0_to_1: f64) -> Self {
        self.uptime_0_to_1 = uptime_0_to_1;
        self
    }
}

/// builder for a set of peers we know about
/// this should generally either be our entire storage arc of peer info
/// or, if it is huge, small sample arc
pub struct RValuePeerRecordSet {
    arc_of_included_peer_records: Arc,
    peer_records: Vec<RValuePeerRecord>,
}

impl Default for RValuePeerRecordSet {
    fn default() -> Self {
        Self {
            arc_of_included_peer_records: Arc::new(0.into(), 0),
            peer_records: Vec::new(),
        }
    }
}

impl RValuePeerRecordSet {
    /// the arc we are representing with this PeerRecordSet
    pub fn arc_of_included_peer_records(mut self, arc_of_included_peer_records: Arc) -> Self {
        self.arc_of_included_peer_records = arc_of_included_peer_records;
        self
    }

    /// push a new PeerRecord into this record set
    pub fn push_peer_record(mut self, peer_record: RValuePeerRecord) -> Self {
        if !self
            .arc_of_included_peer_records
            .contains_location(peer_record.storage_arc.center())
        {
            panic!("peer record does not fit within arc_of_included_peer_records");
        }
        self.peer_records.push(peer_record);
        self
    }
}

/// Interpolate qualified redundancy in the network given our current
/// knowledge of peers' claimed storage arcs.
/// For a given storage arc (should match *this* agent's storage arc),
/// if all the peers we know about are actually storing what they claim to be
/// we can interpolate what the whole network "R" value is
/// also taking into account our experience of those agents' reachability.
pub fn interpolate_r_value_for_given_arc(peer_record_set: &RValuePeerRecordSet) -> f64 {
    let mut running_pct_total: f64 = 0.0;

    for record in peer_record_set.peer_records.iter() {
        // first get the percentage of total space this peer is covering
        let mut relative_pct_of_space_covered: f64 =
            record.storage_arc.length() as f64 / ARC_LENGTH_MAX as f64;

        // if their uptime is less than 1,
        // they only count directly proportional to their uptime
        relative_pct_of_space_covered *= record.uptime_0_to_1;

        // update our running percentage covered
        running_pct_total += relative_pct_of_space_covered;
    }

    let pct_of_space_covered: f64 =
        peer_record_set.arc_of_included_peer_records.length() as f64 / ARC_LENGTH_MAX as f64;

    // we are only sampling the pct_of_space_covered
    // we need to grow our sample (interpolate) to the rest of the space
    running_pct_total * (1.0 / pct_of_space_covered)
}

/// As an agent, we need to set our storage arc length to something
/// on the one hand, if we have enough storage and network bandwidth,
/// we might want to set it as high as possible. On the other hand, we
/// might want to conserve resources / power. If our network is healthy,
/// we can scale back a bit to conserve, if it is in danger, we may want
/// to scale up for safety. If it is anywhere in the middle, we probably
/// want to maintain stability so we're not thrashing our resources.
///
/// Given current network conditions (within the sample slice given)
/// this function will return a recommended storage arc length.
/// LENGTH, not radius. Please divide by 2 if using Arc::new_radius.
///
/// If you provide a current_arc_length, and the network is anywhere near
/// stable, this function will probably return your current length back.
/// Otherwise it will pick a maintenance target right in the middle.
///
/// If the network is over replicated, this function may pick a small length.
/// If the network is unhealthy, or immature this function may pick a large
/// arc length, up to 100% in the case of a new / immature network.
///
/// Also, this is a bit of a naive first implementation, expect the specific
/// algorithm / heuristics to be updated on an on-going basis.
pub fn get_recommended_storage_arc_length(
    peer_record_set: &RValuePeerRecordSet,
    target_minimum_r_value: f64,
    target_maximum_r_value: f64,
    self_uptime_0_to_1: f64,
    current_arc_length: Option<u64>,
) -> u64 {
    let pct_of_space_covered: f64 =
        peer_record_set.arc_of_included_peer_records.length() as f64 / ARC_LENGTH_MAX as f64;
    let interp_total_node_count =
        peer_record_set.peer_records.len() as f64 * (1.0 / pct_of_space_covered);
    if interp_total_node_count < target_minimum_r_value * 2.0 {
        // consider this network immature, recommend full coverage
        return ARC_LENGTH_MAX;
    }

    let cur_r_value = interpolate_r_value_for_given_arc(peer_record_set);

    let mut count: u64 = 0;
    let mut len_min: u64 = ARC_LENGTH_MAX;
    let mut len_max: u64 = 0;
    let mut len_avg: f64 = 0.0;

    #[allow(clippy::explicit_counter_loop)]
    for record in peer_record_set.peer_records.iter() {
        count += 1;

        let len = record.storage_arc.length();

        len_min = std::cmp::min(len, len_min);
        len_max = std::cmp::max(len, len_max);
        len_avg = ((len_avg * (count as f64 - 1.0)) + len as f64) / count as f64;
    }

    /*
    println!(r#"
        count: {}
        len_min: {}
        len_max: {}
        len_avg: {}
    "#, count, len_min, len_max, len_avg);
    */

    let out_length: f64 = if cur_r_value < target_minimum_r_value {
        // our network is unhealthy! let's try to capture more!
        let new_mid_len = len_avg * 1.375;
        match current_arc_length {
            None => new_mid_len,
            Some(ucur) => {
                // again - try to maintain some stability
                let cur = ucur as f64;
                let new_min_len = len_avg * 1.25;
                let new_max_len = len_avg * 1.5;
                if cur > new_min_len && cur < new_max_len {
                    return ucur;
                } else {
                    new_mid_len
                }
            }
        }
    } else if cur_r_value > target_maximum_r_value {
        // our network is heavy! let's pull back
        let new_mid_len = len_avg * 0.625;
        match current_arc_length {
            None => new_mid_len,
            Some(ucur) => {
                // again - try to maintain some stability
                let cur = ucur as f64;
                let new_min_len = len_avg * 0.5;
                let new_max_len = len_avg * 0.75;
                if cur > new_min_len && cur < new_max_len {
                    return ucur;
                } else {
                    new_mid_len
                }
            }
        }
    } else {
        // our network is perfect, let's try to keep it this way!
        match current_arc_length {
            None => len_avg,
            Some(cur) => return cur,
        }
    };

    // correct for our own uptime
    (out_length * (1.0 / self_uptime_0_to_1)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_interpolate_r_value_for_small_sample_count() {
        let set = RValuePeerRecordSet::default()
            .arc_of_included_peer_records(Arc::new(0.into(), ARC_LENGTH_MAX))
            .push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0.into(), ARC_LENGTH_MAX))
                    .uptime_0_to_1(0.5),
            )
            .push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0x80000000.into(), ARC_LENGTH_MAX))
                    .uptime_0_to_1(0.5),
            );

        assert_eq!(1.0, interpolate_r_value_for_given_arc(&set),);
    }

    #[test]
    fn it_can_interpolate_r_value_for_large_sample_count() {
        let mut set =
            RValuePeerRecordSet::default().arc_of_included_peer_records(Arc::new(42.into(), 100));
        for i in 42..142 {
            set = set.push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new_radius(i.into(), 100))
                    .uptime_0_to_1(0.5),
            );
        }

        assert_eq!(99.5, interpolate_r_value_for_given_arc(&set),);
    }

    #[test]
    fn it_can_get_recommended_arc_length_immature() {
        let set = RValuePeerRecordSet::default()
            .arc_of_included_peer_records(Arc::new(0.into(), ARC_LENGTH_MAX))
            .push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0.into(), 42))
                    .uptime_0_to_1(0.5),
            )
            .push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0x80000000.into(), 88))
                    .uptime_0_to_1(0.5),
            );

        // network is immature - algorithm recommends full coverage
        assert_eq!(
            ARC_LENGTH_MAX,
            get_recommended_storage_arc_length(&set, 25.0, 50.0, 0.8, Some(ARC_LENGTH_MAX / 4),),
        );
    }

    #[test]
    fn it_can_get_recommended_arc_length_low() {
        let mut set = RValuePeerRecordSet::default()
            .arc_of_included_peer_records(Arc::new(0.into(), ARC_LENGTH_MAX));

        // if average length is 1/4, with 0.75 uptime = 0.1875 avg pct
        // we need ~90 nodes to make an unhealthy (low coverage) network
        for _ in 0..90 {
            set = set.push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0.into(), ARC_LENGTH_MAX / 4))
                    .uptime_0_to_1(0.75),
            )
        }

        assert!(
            get_recommended_storage_arc_length(&set, 25.0, 50.0, 0.8, Some(ARC_LENGTH_MAX / 4),)
                > ARC_LENGTH_MAX / 4
        );
    }

    #[test]
    fn it_can_get_recommended_arc_length_medium() {
        let mut set = RValuePeerRecordSet::default()
            .arc_of_included_peer_records(Arc::new(0.into(), ARC_LENGTH_MAX));

        // if average length is 1/4, with 0.75 uptime = 0.1875 avg pct
        // we need ~203 nodes to make a medium healthy network
        for _ in 0..203 {
            set = set.push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0.into(), ARC_LENGTH_MAX / 4))
                    .uptime_0_to_1(0.75),
            )
        }

        assert_eq!(
            ARC_LENGTH_MAX / 4,
            get_recommended_storage_arc_length(&set, 25.0, 50.0, 0.8, Some(ARC_LENGTH_MAX / 4),),
        );
    }

    #[test]
    fn it_can_get_recommended_arc_length_heavy() {
        let mut set = RValuePeerRecordSet::default()
            .arc_of_included_peer_records(Arc::new(0.into(), ARC_LENGTH_MAX));

        // if average length is 1/4, with 0.75 uptime = 0.1875 avg pct
        // ~1066 nodes will give us an r-value around 200
        for _ in 0..1066 {
            set = set.push_peer_record(
                RValuePeerRecord::default()
                    .storage_arc(Arc::new(0.into(), ARC_LENGTH_MAX / 4))
                    .uptime_0_to_1(0.75),
            )
        }

        assert!(
            get_recommended_storage_arc_length(&set, 25.0, 50.0, 0.8, Some(ARC_LENGTH_MAX / 4),)
                < ARC_LENGTH_MAX / 4
        );
    }
}
