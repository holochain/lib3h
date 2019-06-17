use crate::Buffer;

/// a helper object that will automatically secure a SecBuf when dropped
pub struct ReadLocker<'a, T: Buffer>(&'a T);

impl<'a, T: Buffer> ReadLocker<'a, T> {
    pub fn new(b: &'a T) -> Self {
        b.set_readable();
        ReadLocker(b)
    }
}

impl<'a, T: Buffer> Drop for ReadLocker<'a, T> {
    fn drop(&mut self) {
        self.0.set_no_access();
    }
}

impl<'a, T: Buffer> std::fmt::Debug for ReadLocker<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", *self.0)
    }
}

impl<'a, T: Buffer> std::ops::Deref for ReadLocker<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}
