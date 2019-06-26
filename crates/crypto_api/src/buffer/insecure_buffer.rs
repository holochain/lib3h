use crate::{Buffer, BufferType, CryptoResult, ProtectState};

/// You probably just want to use Vec<u8> directly rather than this.
/// This is a class is mainly an implementation reference for SecureBuffers.
#[derive(Debug, Clone)]
pub struct InsecureBuffer {
    b: Box<[u8]>,
    p: std::cell::RefCell<ProtectState>,
}

impl std::ops::Deref for InsecureBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("Deref, but state is NoAccess");
        }
        &self.b
    }
}

impl std::ops::DerefMut for InsecureBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if *self.p.borrow() != ProtectState::ReadWrite {
            panic!("DerefMut, but state is not ReadWrite");
        }
        &mut self.b
    }
}

impl BufferType for InsecureBuffer {}

impl Buffer for InsecureBuffer {
    fn new(size: usize) -> CryptoResult<Self> {
        Ok(InsecureBuffer {
            b: vec![0; size].into_boxed_slice(),
            p: std::cell::RefCell::new(ProtectState::NoAccess),
        })
    }

    fn len(&self) -> usize {
        self.b.len()
    }

    fn set_no_access(&self) {
        *self.p.borrow_mut() = ProtectState::NoAccess;
    }

    fn set_readable(&self) {
        *self.p.borrow_mut() = ProtectState::ReadOnly;
    }

    fn set_writable(&self) {
        *self.p.borrow_mut() = ProtectState::ReadWrite;
    }
}