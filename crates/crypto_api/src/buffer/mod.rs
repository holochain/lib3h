use super::{CryptoError, CryptoResult};

pub mod read_lock;
use read_lock::ReadLocker;

pub mod write_lock;
use write_lock::WriteLocker;

pub mod insecure_buffer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectState {
    NoAccess,
    ReadOnly,
    ReadWrite,
}

/// This is a thunk so we don't have to type these trait bounds over and over
pub trait BufferType:
    Sized
    + Send
    + Clone
    + std::fmt::Debug
    + std::ops::Deref<Target = [u8]>
    + std::ops::DerefMut<Target = [u8]>
{
}

pub trait Buffer: BufferType {
    fn new(size: usize) -> CryptoResult<Self>;

    fn len(&self) -> usize;
    fn set_no_access(&self);
    fn set_readable(&self);
    fn set_writable(&self);

    fn read_lock(&self) -> ReadLocker<Self> {
        ReadLocker::new(self)
    }

    fn write_lock(&mut self) -> WriteLocker<Self> {
        WriteLocker::new(self)
    }

    fn write(&mut self, offset: usize, data: &[u8]) -> CryptoResult<()> {
        if offset + data.len() > self.len() {
            return Err(CryptoError::new("write overflow"));
        }
        unsafe {
            let mut b = self.write_lock();
            std::ptr::copy(data.as_ptr(), (**b).as_mut_ptr().add(offset), data.len());
        }
        Ok(())
    }
}

impl BufferType for Vec<u8> {}

impl Buffer for Vec<u8> {
    fn new(size: usize) -> CryptoResult<Self> {
        Ok(vec![0; size])
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn set_no_access(&self) {}
    fn set_readable(&self) {}
    fn set_writable(&self) {}
}
