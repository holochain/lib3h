extern crate backtrace;
#[macro_use]
extern crate detach;
extern crate hcid;
extern crate lib3h_crypto_api;
extern crate lib3h_protocol;
extern crate lib3h_zombie_actor as lib3h_ghost_actor;
extern crate nanoid;
extern crate native_tls;
extern crate tungstenite;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde;
extern crate serde_json;
#[macro_use]
extern crate log;

// -- mod -- //

pub mod dht;
pub mod engine;
pub mod error;
pub mod gateway;
pub mod keystore;
pub mod time;
pub mod track;
pub mod transport;
// FIXME
// pub mod transport_wss;

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
