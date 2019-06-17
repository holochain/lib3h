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

/// This is a thunk so we don't have to type these trait bounds over and over
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

pub trait CryptoSystem {
    type SigPrivKey;
    type EncPrivKey;
    type PwHashConfig;

    const SIG_SEED_SIZE: usize;
    const SIG_SIZE: usize;
    const ENC_SEED_SIZE: usize;

    fn random<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()>;

    fn signature_from_seed<SeedBuffer: Buffer>(&self, seed: &SeedBuffer) -> CryptoResult<Self::SigPrivKey>;

    fn encryption_from_seed<SeedBuffer: Buffer>(&self, seed: &SeedBuffer) -> CryptoResult<Self::EncPrivKey>;

    fn password_hash<InputBuffer: Buffer, OutputBuffer: Buffer>(&self, input: &InputBuffer, output: &mut OutputBuffer, config: &Self::PwHashConfig) -> CryptoResult<()>;
}

pub trait SignaturePublicKey: Sized {
    fn get_id(&self) -> CryptoResult<&str>;
    fn verify<SigBuffer: Buffer, DataBuffer: Buffer>(&self, signature: &SigBuffer, data: &DataBuffer) -> CryptoResult<bool>;
}

pub trait SignaturePrivateKey: Sized {
    type SigPubKey;

    fn get_public_key(&self) -> CryptoResult<Self::SigPubKey>;
    fn sign<SigBuffer: Buffer, DataBuffer: Buffer>(&self, signature: &mut SigBuffer, data: &DataBuffer) -> CryptoResult<()>;
}

pub trait EncryptionPublicKey: Sized {
    fn get_id(&self) -> CryptoResult<&str>;
}

pub trait EncryptionPrivateKey: Sized {
    type EncPubKey;

    fn get_public_key(&self) -> CryptoResult<Self::EncPubKey>;
}

// -- Fake -- //

#[derive(Clone)]
pub struct FakeSignaturePublicKey(pub String);

impl SignaturePublicKey for FakeSignaturePublicKey {
    fn get_id(&self) -> CryptoResult<&str> {
        Ok(&self.0)
    }

    fn verify<SigBuffer: Buffer, DataBuffer: Buffer>(&self, _signature: &SigBuffer, _data: &DataBuffer) -> CryptoResult<bool> {
        Ok(true)
    }
}

pub struct FakeSignaturePrivateKey {
    public_key: FakeSignaturePublicKey,
}

impl SignaturePrivateKey for FakeSignaturePrivateKey {
    type SigPubKey = FakeSignaturePublicKey;

    fn get_public_key(&self) -> CryptoResult<Self::SigPubKey> {
        Ok(self.public_key.clone())
    }

    fn sign<SigBuffer: Buffer, DataBuffer: Buffer>(&self, signature: &mut SigBuffer, _data: &DataBuffer) -> CryptoResult<()> {
        {
            let mut signature = signature.write_lock();
            signature[0] = 1;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct FakeEncryptionPublicKey(pub String);

impl EncryptionPublicKey for FakeEncryptionPublicKey {
    fn get_id(&self) -> CryptoResult<&str> {
        Ok(&self.0)
    }
}

pub struct FakeEncryptionPrivateKey {
    public_key: FakeEncryptionPublicKey,
}

impl EncryptionPrivateKey for FakeEncryptionPrivateKey {
    type EncPubKey = FakeEncryptionPublicKey;

    fn get_public_key(&self) -> CryptoResult<Self::EncPubKey> {
        Ok(self.public_key.clone())
    }
}

pub struct FakePwHashConfig();

pub struct FakeCryptoSystem {}

impl CryptoSystem for FakeCryptoSystem {
    type SigPrivKey = FakeSignaturePrivateKey;
    type EncPrivKey = FakeEncryptionPrivateKey;
    type PwHashConfig = FakePwHashConfig;

    const SIG_SEED_SIZE: usize = 8;
    const SIG_SIZE: usize = 8;
    const ENC_SEED_SIZE: usize = 8;

    fn random<OutputBuffer: Buffer>(&self, buffer: &mut OutputBuffer) -> CryptoResult<()> {
        {
            let mut buffer = buffer.write_lock();
            buffer[0] = 1;
        }

        Ok(())
    }

    fn signature_from_seed<SeedBuffer: Buffer>(&self, _seed: &SeedBuffer) -> CryptoResult<Self::SigPrivKey> {
        Ok(FakeSignaturePrivateKey {
            public_key: FakeSignaturePublicKey("fake".to_string()),
        })
    }

    fn encryption_from_seed<SeedBuffer: Buffer>(&self, _seed: &SeedBuffer) -> CryptoResult<Self::EncPrivKey> {
        Ok(FakeEncryptionPrivateKey {
            public_key: FakeEncryptionPublicKey("fake".to_string()),
        })
    }

    fn password_hash<InputBuffer: Buffer, OutputBuffer: Buffer>(&self, _input: &InputBuffer, _output: &mut OutputBuffer, _config: &Self::PwHashConfig) -> CryptoResult<()> {
        Ok(())
    }
}
