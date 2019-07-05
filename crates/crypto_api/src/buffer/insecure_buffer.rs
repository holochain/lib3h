use std::ops::{Deref, DerefMut};

use crate::{Buffer, ProtectState};

#[derive(Debug, Clone)]
pub struct InsecureBuffer {
    b: Box<[u8]>,
    p: std::cell::RefCell<ProtectState>,
}

impl InsecureBuffer {
    pub fn new(size: usize) -> Self {
        InsecureBuffer {
            b: vec![0; size].into_boxed_slice(),
            p: std::cell::RefCell::new(ProtectState::NoAccess),
        }
    }
}

impl Deref for InsecureBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("Deref, but state is NoAccess");
        }
        &self.b
    }
}

impl DerefMut for InsecureBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if *self.p.borrow() != ProtectState::ReadWrite {
            panic!("DerefMut, but state is not ReadWrite");
        }
        &mut self.b
    }
}

impl Buffer for InsecureBuffer {
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
        self.b.len()
    }
    fn set_no_access(&self) {
        if *self.p.borrow() == ProtectState::NoAccess {
            panic!("already no access... bad logic");
        }
        *self.p.borrow_mut() = ProtectState::NoAccess;
    }
    fn set_readable(&self) {
        if *self.p.borrow() != ProtectState::NoAccess {
            panic!("not no access... bad logic");
        }
        *self.p.borrow_mut() = ProtectState::ReadOnly;
    }
    fn set_writable(&self) {
        if *self.p.borrow() != ProtectState::NoAccess {
            panic!("not no access... bad logic");
        }
        *self.p.borrow_mut() = ProtectState::ReadWrite;
    }
}
