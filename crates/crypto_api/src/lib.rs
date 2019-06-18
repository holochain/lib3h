//! lib3h abstract cryptography traits and data types

#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod error;
pub use error::{CryptoError, CryptoResult};

mod buffer;
pub use buffer::{
    insecure_buffer::InsecureBuffer, read_lock::ReadLocker, write_lock::WriteLocker, Buffer,
    BufferType, ProtectState,
};

mod random;
pub use random::CryptoRandom;

mod sign;
pub use sign::CryptoSignature;

mod system;
pub use system::CryptoSystem;

mod fake_system;
pub use fake_system::FakeCryptoSystem;
