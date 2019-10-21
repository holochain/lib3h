use crate::rrdht_util::Location;

pub const ARC_LENGTH_MAX: u64 = 0x100000000;
pub const ARC_RADIUS_MAX: u32 = 0x80000001;

/// An rrdht "arc" indicates a range on the u32 "location" circle
/// In general, to implement the rrdht sharding algorithm, new `Arc`s
/// will need to be created using `Arc::new_radius(center, radius)`.
/// The `center` parameter for a storage arc, for example, will be the
/// agent's u32 "Location", and the `radius` (read: half arc-length)
/// will be how large an arc around that center location the agent
/// is claiming to store.
/// - 0 indicates they are storing nothing
/// - 1 indicates they are storing only those entries that hash to the
///     exact same u32 location value
/// - 2 indicates they store that same u32 value + one on each side, etc.
/// - ARC_RADIUS_MAX indicates they claim to store all values.
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
    /// if length > ARC_LENGTH_MAX it will be clipped to that value
    pub fn new(start: Location, length: u64) -> Self {
        let length = std::cmp::min(length, ARC_LENGTH_MAX);
        Self { start, length }
    }

    /// construct a new Arc instance given a center Location and a radius
    /// radius as in half the arc length
    pub fn new_radius(center: Location, radius: u32) -> Self {
        if radius == 0 {
            return Arc::new(center, 0);
        }

        let radius = std::cmp::min(radius, ARC_RADIUS_MAX);

        // we need to subtract 1 from the radius in the following
        // calculations to ensure that:
        // - 0 means no coverage
        // - 1 means coverage only including the center point
        // - any thing obove expand out to the sides
        let start = center - Location::from(radius - 1);
        let length = u64::from(radius) * 2 - 1;

        Arc::new(start, length)
    }

    /// construct a new Arc from canonical BITSrHEX:HEX format
    pub fn new_repr(repr: &str) -> Self {
        match Arc::try_new_repr(repr) {
            Err(e) => panic!(e),
            Ok(a) => a,
        }
    }

    /// construct a new Arc from canonical BITSrHEX:HEX format
    pub fn try_new_repr(repr: &str) -> crate::error::Lib3hResult<Self> {
        macro_rules! err {
            () => {
                crate::error::Lib3hError::from(format!("could not parse arc repr: {:?}", repr))
            };
        }
        if "32r" != &repr[0..3] {
            return Err(err!());
        }
        let loc = u32::from_str_radix(&repr[3..11], 16).map_err(|_| err!())?;
        let len = u64::from_str_radix(&repr[12..], 16).map_err(|_| err!())?;
        Ok(Self {
            start: loc.into(),
            length: len,
        })
    }

    /// the start position for this arc
    pub fn start(&self) -> Location {
        self.start
    }

    /// the length of this arc
    pub fn length(&self) -> u64 {
        self.length
    }

    /// the center point of this arc, the "location" if specified by radius
    pub fn center(&self) -> Location {
        if self.length == 0 {
            return self.start;
        }
        // see "new_radius" for description of why the -1
        self.start + (self.radius() - 1).into()
    }

    /// the radius of this arc
    pub fn radius(&self) -> u32 {
        if self.length == 0 {
            return 0;
        }
        // in "new_radius" we had to subtract 1 to manage defining an arc
        // around a center point.
        // To get the given radius we need to add the 1 back in
        (self.length / 2 + 1) as u32
    }

    /// returns `true` if given location is within this arc
    pub fn contains_location(&self, location: Location) -> bool {
        u64::from(self.start.forward_distance_to(location)) < self.length
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
    fn it_can_parse() {
        let a = Arc::new(0xffffffff.into(), ARC_LENGTH_MAX);
        let b = a.to_string();
        assert_eq!("32rffffffff:100000000", b);
        let c = Arc::new_repr(&b);
        assert_eq!(a, c);
        let d = Arc::new(0xffffffff.into(), 0);
        let e = d.to_string();
        assert_eq!("32rffffffff:000000000", e);
        let f = Arc::new_repr(&e);
        assert_eq!(d, f);
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
    fn it_center_and_radius_give_sane_results() {
        // make sure zero length works
        let t = Arc::new(42.into(), 0);
        assert_eq!(42, t.center());
        assert_eq!(42, t.start());
        assert_eq!(0, t.length());
        assert_eq!(0, t.radius());
        let t = Arc::new(43.into(), 0);
        assert_eq!(43, t.center());
        assert_eq!(43, t.start());
        assert_eq!(0, t.length());
        assert_eq!(0, t.radius());
        let t = Arc::new_radius(42.into(), 0);
        assert_eq!(42, t.center());
        assert_eq!(42, t.start());
        assert_eq!(0, t.length());
        assert_eq!(0, t.radius());
        let t = Arc::new_radius(43.into(), 0);
        assert_eq!(43, t.center());
        assert_eq!(43, t.start());
        assert_eq!(0, t.length());
        assert_eq!(0, t.radius());
        // make sure 1 length works
        let t = Arc::new(42.into(), 1);
        assert_eq!(42, t.center());
        assert_eq!(42, t.start());
        assert_eq!(1, t.length());
        assert_eq!(1, t.radius());
        let t = Arc::new(43.into(), 1);
        assert_eq!(43, t.center());
        assert_eq!(43, t.start());
        assert_eq!(1, t.length());
        assert_eq!(1, t.radius());
        // make sure 1 radius works
        let t = Arc::new_radius(42.into(), 1);
        assert_eq!(42, t.center());
        assert_eq!(42, t.start());
        assert_eq!(1, t.length());
        assert_eq!(1, t.radius());
        let t = Arc::new_radius(43.into(), 1);
        assert_eq!(43, t.center());
        assert_eq!(43, t.start());
        assert_eq!(1, t.length());
        assert_eq!(1, t.radius());
        // 2 length is a little weird... it's not an even radius
        let t = Arc::new(42.into(), 2);
        assert_eq!(43, t.center());
        assert_eq!(42, t.start());
        assert_eq!(2, t.length());
        assert_eq!(2, t.radius());
        let t = Arc::new(43.into(), 2);
        assert_eq!(44, t.center());
        assert_eq!(43, t.start());
        assert_eq!(2, t.length());
        assert_eq!(2, t.radius());
        // make sure 3 length works
        let t = Arc::new(42.into(), 3);
        assert_eq!(43, t.center());
        assert_eq!(42, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
        let t = Arc::new(43.into(), 3);
        assert_eq!(44, t.center());
        assert_eq!(43, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
        // make sure 2 radius works
        let t = Arc::new_radius(42.into(), 2);
        assert_eq!(42, t.center());
        assert_eq!(41, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
        let t = Arc::new_radius(43.into(), 2);
        assert_eq!(43, t.center());
        assert_eq!(42, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
        // make sure full length works
        let t = Arc::new(42.into(), ARC_LENGTH_MAX);
        assert_eq!(0x8000002a, t.center());
        assert_eq!(42, t.start());
        assert_eq!(ARC_LENGTH_MAX, t.length());
        assert_eq!(ARC_RADIUS_MAX, t.radius());
        let t = Arc::new(43.into(), ARC_LENGTH_MAX);
        assert_eq!(0x8000002b, t.center());
        assert_eq!(43, t.start());
        assert_eq!(ARC_LENGTH_MAX, t.length());
        assert_eq!(ARC_RADIUS_MAX, t.radius());
        // make sure full radius works
        let t = Arc::new_radius(42.into(), ARC_RADIUS_MAX);
        assert_eq!(42, t.center());
        assert_eq!(0x8000002a, t.start());
        assert_eq!(ARC_LENGTH_MAX, t.length());
        assert_eq!(ARC_RADIUS_MAX, t.radius());
        let t = Arc::new_radius(43.into(), ARC_RADIUS_MAX);
        assert_eq!(43, t.center());
        assert_eq!(0x8000002b, t.start());
        assert_eq!(ARC_LENGTH_MAX, t.length());
        assert_eq!(ARC_RADIUS_MAX, t.radius());
        // make sure wrap length works
        let t = Arc::new(0xffffffff.into(), 3);
        assert_eq!(0, t.center());
        assert_eq!(0xffffffff, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
        // make sure wrap radius works
        let t = Arc::new_radius(0.into(), 2);
        assert_eq!(0, t.center());
        assert_eq!(0xffffffff, t.start());
        assert_eq!(3, t.length());
        assert_eq!(2, t.radius());
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
