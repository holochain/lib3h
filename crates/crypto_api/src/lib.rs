//! lib3h abstract cryptography traits and data types
//!
//! # Examples
//!
//! ```
//! extern crate lib3h_crypto_api;
//!
//! fn main() {
//! }
//! ```

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate zeroize;

mod error;
pub use error::{CryptoError, CryptoResult};

mod buffer;
pub use buffer::{Buffer, ProtectState, ReadLocker, WriteLocker};

mod crypto_system;
pub use crypto_system::{crypto_system_test, CryptoSystem};
