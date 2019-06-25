use super::{CryptoError, CryptoResult};

pub mod read_lock;
use read_lock::ReadLocker;

pub mod write_lock;
use write_lock::WriteLocker;

pub mod insecure_buffer;

/// Track if a buffer has read/write access or is memory protected.
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

/// The Buffer trait is used by crypto_api functions to exchange data.
/// It is implemented for Vec<u8> for direct use.
/// If your crypto system provides memory security, you should prefer that type for private keys.
pub trait Buffer: BufferType {
    /// Create a new Buffer instance of given type
    fn new(size: usize) -> CryptoResult<Self>;

    /// Get the length of this buffer
    fn len(&self) -> usize;

    /// Mark the buffer as no-access (secure)
    fn set_no_access(&self);

    /// Mark the buffer as read-only (see read_lock)
    fn set_readable(&self);

    /// Mark the buffer as read-write (see write_lock)
    fn set_writable(&self);

    /// Mark the buffer as readable (read-only)
    /// When the returned ReadLocker instance is dropped,
    /// the buffer will be marked no-access.
    fn read_lock(&self) -> ReadLocker<Self> {
        ReadLocker::new(self)
    }

    /// Mark the buffer as writable (read-write)
    /// When the returned WriteLocker instance is dropped,
    /// the buffer will be marked no-access.
    fn write_lock(&mut self) -> WriteLocker<Self> {
        WriteLocker::new(self)
    }

    /// Write data to this Buffer instance
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

// implement our base thunk for Vec<u8>
impl BufferType for Vec<u8> {}

// implement the Buffer trait for Vec<u8>
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
