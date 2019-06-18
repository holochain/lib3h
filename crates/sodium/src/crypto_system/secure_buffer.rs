use lib3h_crypto_api::{Buffer, BufferType, CryptoError, CryptoResult, ProtectState};

use crate::check_init;
use libc::c_void;

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
        let mut out = SecureBuffer::new(self.s).expect("could not alloc new");
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

    fn deref(&self) -> &[u8] {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("Deref, but state is NoAccess");
        }
        unsafe { std::slice::from_raw_parts(self.z as *const u8, self.s) }
    }
}

impl std::ops::DerefMut for SecureBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        if *self.p.borrow() != ProtectState::ReadWrite {
            panic!("DerefMut, but state is not ReadWrite");
        }
        unsafe { std::slice::from_raw_parts_mut(self.z as *mut u8, self.s) }
    }
}

impl BufferType for SecureBuffer {}

impl Buffer for SecureBuffer {
    fn new(size: usize) -> CryptoResult<Self> {
        check_init();
        let z = unsafe {
            let z = rust_sodium_sys::sodium_malloc(size);
            if z.is_null() {
                return Err(CryptoError::new("memory error"));
            }
            rust_sodium_sys::sodium_mprotect_noaccess(z);
            z
        };

        Ok(SecureBuffer {
            z,
            s: size,
            p: std::cell::RefCell::new(ProtectState::NoAccess),
        })
    }

    fn len(&self) -> usize {
        self.s
    }

    fn set_no_access(&self) {
        unsafe {
            rust_sodium_sys::sodium_mprotect_noaccess(self.z);
        }
        *self.p.borrow_mut() = ProtectState::NoAccess;
    }

    fn set_readable(&self) {
        unsafe {
            rust_sodium_sys::sodium_mprotect_readonly(self.z);
        }
        *self.p.borrow_mut() = ProtectState::ReadOnly;
    }

    fn set_writable(&self) {
        unsafe {
            rust_sodium_sys::sodium_mprotect_readwrite(self.z);
        }
        *self.p.borrow_mut() = ProtectState::ReadWrite;
    }
}
