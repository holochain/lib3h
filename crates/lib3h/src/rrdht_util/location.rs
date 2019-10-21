use std::num::Wrapping;

/// An rrdht "location" refers to a hash of a signing public key
/// compressed down into 32 unsigned bits (u32)
/// We use "Wrapping" math because location offsets or "arcs" wrap...
/// i.e. are mapped onto a circle
#[derive(Shrinkwrap, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct Location(pub Wrapping<u32>);

impl Location {
    /// get the distance from this location to an `other` location
    /// this distance is processed forward taking wrapping into account
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn forward_distance_to(&self, other: Location) -> u32 {
        let a = (self.0).0;
        let b = (other.0).0;
        if b >= a {
            return b - a;
        }
        (0xffffffff - a) + b + 1
    }
}

impl From<Wrapping<u32>> for Location {
    fn from(l: Wrapping<u32>) -> Self {
        Location(l)
    }
}

impl From<u32> for Location {
    fn from(l: u32) -> Self {
        Location(Wrapping(l))
    }
}

impl From<Location> for Wrapping<u32> {
    fn from(l: Location) -> Wrapping<u32> {
        l.0
    }
}

impl From<Location> for u32 {
    fn from(l: Location) -> u32 {
        (l.0).0
    }
}

impl PartialEq<Wrapping<u32>> for Location {
    fn eq(&self, other: &Wrapping<u32>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<u32> for Location {
    fn eq(&self, other: &u32) -> bool {
        (self.0).0 == *other
    }
}

impl PartialEq<Location> for Wrapping<u32> {
    fn eq(&self, other: &Location) -> bool {
        *self == other.0
    }
}

impl PartialEq<Location> for u32 {
    fn eq(&self, other: &Location) -> bool {
        *self == (other.0).0
    }
}

impl std::ops::Sub for Location {
    type Output = Location;

    fn sub(self, other: Location) -> Location {
        Location(self.0 - other.0)
    }
}

impl std::ops::Add for Location {
    type Output = Location;

    fn add(self, other: Location) -> Location {
        Location(self.0 + other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_add() {
        assert_eq!(42, Location::from(10) + Location::from(32));
        assert_eq!(42, Location::from(0xffffffff) + Location::from(43));
    }

    #[test]
    fn it_should_sub() {
        assert_eq!(42, Location::from(52) - Location::from(10));
        assert_eq!(0xffffffff, Location::from(42) - Location::from(43));
    }

    #[test]
    fn it_should_calc_forward_distance_to() {
        // one point to the same point is zero distance
        assert_eq!(0, Location::from(0).forward_distance_to(0.into()));
        // one point to the next is a distance of 1 unit
        assert_eq!(1, Location::from(0).forward_distance_to(1.into()));
        // max to zero is 1 unit
        assert_eq!(1, Location::from(0xffffffff).forward_distance_to(0.into()));
        // zero to max is max units
        assert_eq!(
            0xffffffff,
            Location::from(0).forward_distance_to(0xffffffff.into())
        );
        // max to max - 1 is max units
        assert_eq!(
            0xffffffff,
            Location::from(0xffffffff).forward_distance_to(0xfffffffe.into())
        );
    }
}
