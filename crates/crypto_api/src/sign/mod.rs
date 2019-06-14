use super::CryptoResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectState {
    NoAccess,
    ReadOnly,
    ReadWrite,
}

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

/// a helper object that will automatically secure a SecBuf when dropped
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

pub trait BufferType:
    Sized + Send + Clone + std::fmt::Debug + std::ops::Deref<Target = [u8]> + std::ops::DerefMut<Target = [u8]>
{
}

pub trait Buffer: BufferType {
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
}

#[derive(Debug, Clone)]
pub struct InsecureBuffer {
    b: Box<[u8]>,
    p: ProtectState,
}

impl InsecureBuffer {
    pub fn new(size: usize) -> Self {
        InsecureBuffer {
            b: vec![0; size].into_boxed_slice(),
            p: ProtectState::NoAccess,
        }
    }
}

impl std::ops::Deref for InsecureBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        if self.p == ProtectState::NoAccess {
            panic!("Deref, but state is NoAccess");
        }
        &self.b
    }
}

impl std::ops::DerefMut for InsecureBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        if self.p != ProtectState::ReadWrite {
            panic!("DerefMut, but state is not ReadWrite");
        }
        &mut self.b
    }
}

impl BufferType for InsecureBuffer {}

impl Buffer for InsecureBuffer {
    fn len(&self) -> usize {
        self.b.len()
    }

    fn set_no_access(&self) {}
    fn set_readable(&self) {}
    fn set_writable(&self) {}
}

pub trait CanPasswordHash<S, C> {
    fn calc_hash(input: &mut S, output: &mut S, config: &C) -> CryptoResult<()>;
}

pub trait CanEncryptSymmetric<S, C> {
    fn encrypt_symmetric(input: &mut S, output: &mut S, config: &C) -> CryptoResult<()>;
}

pub trait CanDecryptSymmetric<S, C> {
    fn decrypt_symmetric(input: &mut S, output: &mut S, config: &C) -> CryptoResult<()>;
}

pub type Signature = Vec<u8>;
pub type SignatureRef = [u8];

pub type SignatureData = Vec<u8>;
pub type SignatureDataRef = [u8];

pub trait SignatureSeed<PRIV: CanSign, PUB: CanVerify> {
    fn derive_keypair(&self) -> CryptoResult<(PRIV, PUB)>;
}

pub trait CanVerify {
    fn verify(signature: &SignatureRef, data: &SignatureDataRef) -> CryptoResult<bool>;
}

pub trait CanSign {
    fn sign(data: &SignatureDataRef) -> CryptoResult<Signature>;
}
