use crate::rrdht_util::Location;

pub const ARC_LENGTH_MAX: u64 = 0x100000000;
pub const ARC_RADIUS_MAX: u32 = 0x80000001;

/// An rrdht "arc" indicates a range on the u32 "location" circle
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Arc {
    start: Location,
    // length must be a u64 because we need to be able to represent
    // both a zero length arc with length 0,
    // and also a full arc with length one beyond the u32 max
    length: u64,
}

impl std::fmt::Display for Arc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "32r{:08x}:{:09x}", u32::from(self.start), self.length)
    }
}

impl std::fmt::Debug for Arc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Arc").field(&format!("{}", self)).finish()
    }
}

impl Arc {
    /// construct a new Arc instance given a start Location and a length
    pub fn new(start: Location, length: u64) -> Self {
        let length = std::cmp::min(length, ARC_LENGTH_MAX);
        Self { start, length }
    }

    /// construct a new Arc instance given a center Location and a radius
    pub fn new_radius(center: Location, radius: u32) -> Self {
        if radius == 0 {
            return Arc::new(center, 0);
        }
        let radius = std::cmp::min(radius, ARC_RADIUS_MAX);
        let start = center - Location::from(radius - 1);
        let length = u64::from(radius) * 2 - 1;
        Arc::new(start, length)
    }

    /// construct a new Arc from canonical BITSrHEX:HEX format
    /// UNIMPLEMENTED
    pub fn new_repr(_repr: &str) -> Self {
        unimplemented!();
    }

    /// returns `true` if given location is within this arc
    pub fn contains_location(&self, location: Location) -> bool {
        self.start.forward_distance_to(location) < self.length
    }
}

impl<S: AsRef<str>> From<&S> for Arc {
    fn from(s: &S) -> Arc {
        Arc::new_repr(s.as_ref())
    }
}

impl From<String> for Arc {
    fn from(s: String) -> Arc {
        Arc::new_repr(&s)
    }
}

impl From<Arc> for String {
    fn from(a: Arc) -> String {
        a.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_debug() {
        assert_eq!(
            "Arc(\"32r0000002a:00000002a\")",
            &format!("{:?}", Arc::new(42.into(), 42))
        );
    }

    #[test]
    fn it_should_construct_arcs() {
        // full length arc
        assert_eq!(
            "32r00000000:100000000",
            Arc::new(0.into(), ARC_LENGTH_MAX).to_string(),
        );
        // small length arc
        assert_eq!("32r00000000:00000002a", Arc::new(0.into(), 42).to_string(),);
        // wrapping length arc
        assert_eq!(
            "32rffffffff:00000002a",
            Arc::new(0xffffffff.into(), 42).to_string(),
        );
    }

    #[test]
    fn it_should_construct_arcs_by_radius() {
        // zero length arc -- no storage
        assert_eq!(
            "32r0000002a:000000000",
            Arc::new_radius(42.into(), 0).to_string(),
        );
        // 1 length arc -- only covering exactly same location
        assert_eq!(
            "32r0000002a:000000001",
            Arc::new_radius(42.into(), 1).to_string(),
        );
        // 2 length arc -- cover center + 1 on each side
        assert_eq!(
            "32r00000029:000000003",
            Arc::new_radius(42.into(), 2).to_string(),
        );
        // wrapping length arc
        assert_eq!(
            "32rffffffff:000000003",
            Arc::new_radius(0.into(), 2).to_string(),
        );
        // max radius
        assert_eq!(
            "32r80000000:100000000",
            Arc::new_radius(0.into(), ARC_RADIUS_MAX).to_string(),
        );
    }

    #[test]
    fn it_can_calc_contains_location() {
        // zero length should not even contain the same point
        let t = Arc::new(0.into(), 0);
        assert!(!t.contains_location(0.into()));
        // 1 length should contain the same point
        let t = Arc::new(0.into(), 1);
        assert!(t.contains_location(0.into()));
        // 2 length should contain properly wrapping
        let t = Arc::new(0xffffffff.into(), 2);
        assert!(!t.contains_location(0xfffffffe.into()));
        assert!(t.contains_location(0xffffffff.into()));
        assert!(t.contains_location(0.into()));
        assert!(!t.contains_location(1.into()));
    }
}
