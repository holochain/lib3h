use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

use crate::{CryptoError, CryptoResult};

mod read_locker;
pub use read_locker::ReadLocker;

mod write_locker;
pub use write_locker::WriteLocker;

mod buffer_vec_u8;

/// The Buffer trait is used by crypto_api functions to exchange data.
/// It is implemented for Vec<u8> for direct use.
/// For private keys, prefer the type returned by CryptoSystem::buf_new_secure
pub trait Buffer: Send + std::fmt::Debug + Deref<Target = [u8]> + DerefMut<Target = [u8]> {
    /// Buffer is designed to be used as a trait-object
    /// Since we can't get a sized clone, provide clone in a Box.
    fn box_clone(&self) -> Box<dyn Buffer>;

    /// helps work around some sizing issues with rust trait-objects
    fn as_buffer(&self) -> &dyn Buffer;

    /// helps work around some sizing issues with rust trait-objects
    fn as_buffer_mut(&mut self) -> &mut dyn Buffer;

    /// get the length of the buffer
    fn len(&self) -> usize;

    /// is this a zero-length buffer?
    fn is_empty(&self) -> bool;

    /// mark this buffer as no-access memory protection
    /// (note, this is a no-op for Vec<u8>s)
    fn set_no_access(&self);

    /// mark this buffer as read-only memory protection
    /// (note, this is a no-op for Vec<u8>s)
    fn set_readable(&self);

    /// mark this buffer as read-write memory protection
    /// (note, this is a no-op for Vec<u8>s)
    fn set_writable(&self);

    /// return a locker object that marks this Buffer readable
    /// until the locker goes out of scope
    fn read_lock(&self) -> ReadLocker {
        ReadLocker::new(self.as_buffer())
    }

    /// return a locker object that marks this Buffer writable
    /// until the locker goes out of scope
    fn write_lock(&mut self) -> WriteLocker {
        WriteLocker::new(self.as_buffer_mut())
    }

    /// fill the buffer with zeroes
    fn zero(&mut self) {
        // use `zeroize` to ensure this doesn't get optimized out
        self.write_lock().zeroize();
    }

    /// write `data` into this buffer at given `offset`
    /// this function will return CryptoError::WriteOverflow if data is too long
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

    /// compare this buffer to another buffer
    /// Return :
    /// | if a > b; return 1
    /// | if a < b; return -1
    /// | if a == b; return 0
    #[allow(clippy::borrowed_box)]
    fn compare(&self, b: &Box<dyn Buffer>) -> i32 {
        let a = self.read_lock();
        let b = b.read_lock();
        let al = self.len();
        let bl = b.len();
        // Compare al length like libsodium
        for i in (0..al).rev() {
            let av = a[i];
            let bv = if i >= bl { 0 } else { b[i] };
            if av > bv {
                return 1;
            } else if av < bv {
                return -1;
            };
        }
        return 0;
    }
}

/// Track if a buffer has read/write access or is memory protected.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtectState {
    NoAccess,
    ReadOnly,
    ReadWrite,
}
