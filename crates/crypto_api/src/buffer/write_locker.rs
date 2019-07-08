use std::ops::{Deref, DerefMut};

use crate::Buffer;

/// Helper object that will automatically secure a Buffer when dropped
pub struct WriteLocker<'a>(&'a mut dyn Buffer);

impl<'a> WriteLocker<'a> {
    pub fn new(b: &'a mut dyn Buffer) -> Self {
        b.set_writable();
        WriteLocker(b)
    }
}

impl<'a> Drop for WriteLocker<'a> {
    fn drop(&mut self) {
        self.0.set_no_access();
    }
}

impl<'a> std::fmt::Debug for WriteLocker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'a> Deref for WriteLocker<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for WriteLocker<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
