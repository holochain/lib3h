use lib3h_crypto_api::{Buffer, CryptoError, CryptoResult, ProtectState};

use crate::check_init;
use libc::c_void;

/// A secure buffer implementation of lib3h_crypto_api::Buffer
/// making use of libsodium's implementation of mlock and mprotect.
pub struct SecureBuffer {
    z: *mut c_void,
    s: usize,
    p: std::cell::RefCell<ProtectState>,
}

unsafe impl Send for SecureBuffer {}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        unsafe {
            rust_sodium_sys::sodium_free(self.z);
        }
    }
}

impl Clone for SecureBuffer {
    fn clone(&self) -> Self {
        let mut out = SecureBuffer::new(self.s);
        out.write(0, &self.read_lock())
            .expect("could not write new");
        out
    }
}

impl std::fmt::Debug for SecureBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.p.borrow() {
            ProtectState::NoAccess => write!(f, "SecureBuffer( {:?} )", "<NO_ACCESS>"),
            _ => write!(f, "SecureBuffer( {:?} )", *self),
        }
    }
}

impl std::ops::Deref for SecureBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("Deref, but state is NoAccess");
        }
        unsafe { &std::slice::from_raw_parts(self.z as *const u8, self.s)[..self.s] }
    }
}

impl std::ops::DerefMut for SecureBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if *self.p.borrow() != ProtectState::ReadWrite {
            panic!("DerefMut, but state is not ReadWrite");
        }
        unsafe { &mut std::slice::from_raw_parts_mut(self.z as *mut u8, self.s)[..self.s] }
    }
}

impl SecureBuffer {
    pub fn new(size: usize) -> Self {
        check_init();
        let z = unsafe {
            // sodium_malloc requires memory-aligned sizes,
            // round up to the nearest 8 bytes.
            let align_size = (size + 7) & !7;
            let z = rust_sodium_sys::sodium_malloc(align_size);
            if z.is_null() {
                panic!("sodium_malloc could not allocate");
            }
            rust_sodium_sys::sodium_memzero(z, align_size);
            rust_sodium_sys::sodium_mprotect_noaccess(z);
            z
        };

        SecureBuffer {
            z,
            s: size,
            p: std::cell::RefCell::new(ProtectState::NoAccess),
        }
    }
}

impl Buffer for SecureBuffer {
    fn box_clone(&self) -> Box<dyn Buffer> {
        Box::new(self.clone())
    }

    fn as_buffer(&self) -> &dyn Buffer {
        &*self
    }

    fn as_buffer_mut(&mut self) -> &mut dyn Buffer {
        &mut *self
    }

    fn len(&self) -> usize {
        self.s
    }

    fn is_empty(&self) -> bool {
        self.s == 0
    }

    fn set_no_access(&self) {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("already no access... bad logic");
        }
        unsafe {
            rust_sodium_sys::sodium_mprotect_noaccess(self.z);
        }
        *self.p.borrow_mut() = ProtectState::NoAccess;
    }

    fn set_readable(&self) {
        if *self.p.borrow() != ProtectState::NoAccess {
            panic!("not no access... bad logic");
        }
        unsafe {
            rust_sodium_sys::sodium_mprotect_readonly(self.z);
        }
        *self.p.borrow_mut() = ProtectState::ReadOnly;
    }

    fn set_writable(&self) {
        if *self.p.borrow() != ProtectState::NoAccess {
            panic!("not no access... bad logic");
        }
        unsafe {
            rust_sodium_sys::sodium_mprotect_readwrite(self.z);
        }
        *self.p.borrow_mut() = ProtectState::ReadWrite;
    }

    fn compare(&mut self, b: &mut Box<dyn Buffer>) -> i32 {
        let mut a = self.write_lock();
        let mut b = b.write_lock();
        unsafe { rust_sodium_sys::sodium_compare(raw_ptr_char!(a), raw_ptr_char!(b), a.len()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_handles_alignment() {
        // the underlying memory should be 8,
        // but every function should treat it as 1.
        let mut b = SecureBuffer::new(1);
        assert_eq!(1, b.len());
        {
            let r: &[u8] = &b.read_lock()[..];
            assert_eq!(1, r.len());
        }
        {
            let w: &mut [u8] = &mut b.write_lock()[..];
            assert_eq!(1, w.len());
        }
    }
}
