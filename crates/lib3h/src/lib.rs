extern crate hcid;
extern crate lib3h_crypto_api;
extern crate lib3h_protocol;
extern crate nanoid;
extern crate native_tls;
extern crate tungstenite;
extern crate url_serde;
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
#[macro_use]
extern crate unwrap_to;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde;
#[macro_use]
extern crate log;

pub mod error;

use rmp_serde::Deserializer;
use serde::Deserialize;

pub fn lib3h_rmp_deserialize<'a, T: serde::Deserialize<'a>>(
    payload: &[u8],
) -> error::Lib3hResult<T> {
    match std::panic::catch_unwind(|| {
        let mut de = Deserializer::new(payload);
        Deserialize::deserialize(&mut de)
            .map_err(|e| error::Lib3hError::new(error::ErrorKind::RmpSerdeDecodeError(e)))
    }) {
        Ok(r) => r,
        Err(e) => Err(error::Lib3hError::new(error::ErrorKind::Other(format!(
            "{:?}",
            e
        )))),
    }
}

// -- mod -- //

pub mod dht;
pub mod engine;
pub mod gateway;
pub mod time;
pub mod track;
pub mod transport;
pub mod transport_wss;

#[cfg(test)]
pub mod tests {
    // for this to actually show log entries you also have to run the tests like this:
    // RUST_LOG=lib3h=debug cargo test -- --nocapture
    pub fn enable_logging_for_test(enable: bool) {
        // wait a bit because of non monotonic clock,
        // otherwise we could get negative substraction panics
        // TODO #211
        std::thread::sleep(std::time::Duration::from_millis(10));
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "debug");
        }
        let _ = env_logger::builder()
            .default_format_timestamp(false)
            .default_format_module_path(false)
            .is_test(enable)
            .try_init();
    }
}
