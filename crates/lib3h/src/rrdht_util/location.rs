use std::num::Wrapping;

#[derive(Shrinkwrap, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct Location(pub Wrapping<u32>);

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
