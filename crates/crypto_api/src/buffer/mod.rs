use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

use crate::{CryptoError, CryptoResult};

mod read_locker;
pub use read_locker::ReadLocker;

mod write_locker;
pub use write_locker::WriteLocker;

mod buffer_vec_u8;

mod insecure_buffer;
pub use insecure_buffer::InsecureBuffer;

pub trait Buffer: Send + std::fmt::Debug + Deref<Target = [u8]> + DerefMut<Target = [u8]> {
    fn box_clone(&self) -> Box<dyn Buffer>;
    fn as_buffer(&self) -> &dyn Buffer;
    fn as_buffer_mut(&mut self) -> &mut dyn Buffer;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn set_no_access(&self);
    fn set_readable(&self);
    fn set_writable(&self);
    fn read_lock(&self) -> ReadLocker {
        ReadLocker::new(self.as_buffer())
    }
    fn write_lock(&mut self) -> WriteLocker {
        WriteLocker::new(self.as_buffer_mut())
    }
    fn zero(&mut self) {
        self.write_lock().zeroize();
    }
    fn write(&mut self, offset: usize, data: &[u8]) -> CryptoResult<()> {
        if offset + data.len() > self.len() {
            return Err(CryptoError::WriteOverflow);
        }
        unsafe {
            let mut b = self.write_lock();
            std::ptr::copy(data.as_ptr(), (*b).as_mut_ptr().add(offset), data.len());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtectState {
    NoAccess,
    ReadOnly,
    ReadWrite,
}
