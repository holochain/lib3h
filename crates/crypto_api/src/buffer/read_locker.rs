use std::ops::Deref;

use crate::Buffer;

/// Helper object that will automatically secure a Buffer when dropped
pub struct ReadLocker<'a>(&'a dyn Buffer);

impl<'a> ReadLocker<'a> {
    pub fn new(b: &'a dyn Buffer) -> Self {
        b.set_readable();
        ReadLocker(b)
    }
}

impl<'a> Drop for ReadLocker<'a> {
    fn drop(&mut self) {
        self.0.set_no_access();
    }
}

impl<'a> std::fmt::Debug for ReadLocker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'a> Deref for ReadLocker<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
