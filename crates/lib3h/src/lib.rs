#![feature(test)]

extern crate backtrace;
#[macro_use]
extern crate detach;
extern crate hcid;
extern crate lib3h_crypto_api;
extern crate lib3h_p2p_protocol;
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
extern crate shrinkwraprs;
#[macro_use]
extern crate log;

// -- mod -- //

pub mod dht;
pub mod engine;
pub mod error;
pub mod gateway;
pub mod keystore;
pub mod message_encoding;
pub mod rrdht_util;
pub mod time;
pub mod track;
#[macro_use]
pub mod transport;
// FIXME
// pub mod transport_wss;

// Global Tracer
lazy_static! {
    pub static ref LIB3H_TRACER: std::sync::Mutex<holochain_tracing::Tracer> =
        std::sync::Mutex::new(holochain_tracing::null_tracer());
}

pub fn new_root_span(op_name: &str) -> holochain_tracing::Span {
    let tracer = LIB3H_TRACER.lock().unwrap();
    tracer.span(format!("(root) {}", op_name)).start().into()
}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::io::{Read, Seek, Write};

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

    static DATA: &'static [u8] = b"this is some tempfile data";

    fn bench_unit_control_work() {
        // do some io work as a control.
        // this ends up taking a number of milliseconds depending on system
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(DATA).unwrap();
        f.sync_all().unwrap();
        f.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        println!("read from tempfile: {}", String::from_utf8_lossy(&data));
        assert_eq!(data.as_slice(), DATA);
    }

    fn bench_unit_capture_backtrace() {
        bench_unit_control_work();
        let bt = backtrace::Backtrace::new();
        assert_eq!(&format!("{:?}", bt)[0..16], "stack backtrace:");
    }

    fn bench_unit_capture_backtrace_unresolved() {
        bench_unit_control_work();
        let bt = backtrace::Backtrace::new_unresolved();
        assert_eq!(&format!("{:?}", bt)[0..16], "stack backtrace:");
    }

    fn bench_unit_sync_crossbeam() {
        bench_unit_control_work();
        let (send, recv) = crossbeam_channel::unbounded();
        send.send("test-msg".to_string()).unwrap();
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(1));
            match recv.try_recv() {
                Err(_) => continue,
                Ok(s) => {
                    assert_eq!(&s, "test-msg");
                    return;
                }
            }
        }
        panic!("never received msg");
    }

    fn bench_unit_logging() {
        bench_unit_control_work();
        for _ in 0..10 {
            trace!("some trace data: {:?}", "trace data");
            debug!("some debug data: {:?}", "debug data");
            info!("some info data: {:?}", "info data");
            warn!("some warn data: {:?}", "warn data");
            error!("some error data: {:?}", "error data");
        }
    }

    fn bench_helper_use_logging(use_logging: bool) {
        if use_logging {
            std::env::set_var("RUST_LOG", "trace");
        } else {
            std::env::set_var("RUST_LOG", "error");
        }
        let _ = env_logger::builder()
            .default_format_timestamp(false)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_benchmark_functions_should_execute() {
        bench_helper_use_logging(true);
        bench_unit_control_work();
        bench_unit_capture_backtrace();
        bench_unit_capture_backtrace_unresolved();
        bench_unit_sync_crossbeam();
        bench_unit_logging();
    }

    #[bench]
    fn bench_control_work(b: &mut test::Bencher) {
        bench_helper_use_logging(false);
        let cb = || bench_unit_control_work();
        for _ in 0..10 {
            cb();
        }
        b.iter(cb);
    }

    #[bench]
    fn bench_capture_backtrace(b: &mut test::Bencher) {
        bench_helper_use_logging(false);
        let cb = || bench_unit_capture_backtrace();
        for _ in 0..10 {
            cb();
        }
        b.iter(cb);
    }

    #[bench]
    fn bench_capture_backtrace_unresolved(b: &mut test::Bencher) {
        bench_helper_use_logging(false);
        let cb = || bench_unit_capture_backtrace_unresolved();
        for _ in 0..10 {
            cb();
        }
        b.iter(cb);
    }

    #[bench]
    fn bench_sync_crossbeam(b: &mut test::Bencher) {
        bench_helper_use_logging(false);
        let cb = || bench_unit_sync_crossbeam();
        for _ in 0..10 {
            cb();
        }
        b.iter(cb);
    }

    #[bench]
    fn bench_logging(b: &mut test::Bencher) {
        bench_helper_use_logging(true);
        let cb = || bench_unit_logging();
        for _ in 0..10 {
            cb();
        }
        b.iter(cb);
    }
}
