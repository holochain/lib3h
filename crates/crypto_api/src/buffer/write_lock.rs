use crate::Buffer;

/// Helper object that will automatically secure a Buffer when dropped
pub struct WriteLocker<'a, T: Buffer>(&'a mut T);

impl<'a, T: Buffer> WriteLocker<'a, T> {
    pub fn new(b: &'a mut T) -> Self {
        b.set_writable();
        WriteLocker(b)
    }
}

impl<'a, T: Buffer> Drop for WriteLocker<'a, T> {
    fn drop(&mut self) {
        self.0.set_no_access();
    }
}

impl<'a, T: Buffer> std::fmt::Debug for WriteLocker<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", *self.0)
    }
}

impl<'a, T: Buffer> std::ops::Deref for WriteLocker<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T: Buffer> std::ops::DerefMut for WriteLocker<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}
