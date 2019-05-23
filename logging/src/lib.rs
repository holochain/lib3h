//! Implementation of an asynchronous logger for `lib3h`

use chrono;
pub use slog_async;
pub use slog_scope::{self, crit, debug, error, info, trace, warn};
pub use slog_term;

use slog::Drain;
pub use slog::{slog_crit, slog_debug, slog_error, slog_info, slog_o, slog_trace, slog_warn};

use std::{
    fs::OpenOptions,
    path::PathBuf,
};

/// This struct is used as a wrapper around the `slog` crate for our customized logger.
/// This has to be instantiate at the top of the user program in order to keep
/// the logging thread alive.
pub struct Logger(slog_scope::GlobalLoggerGuard);

impl Logger {
    pub fn init() -> Self {
        Logger::init_terminal_logger()
    }

    /// Log directly in the terminal.
    pub fn init_terminal_logger() -> Self {
        Self(slog_scope::set_global_logger(Logger::init_log()))
    }

    /// Initializes the file logging capabilities.
    pub fn init_file_logger(log_file: &str) -> Self {
        Self(slog_scope::set_global_logger(Logger::init_log_to_file(log_file)))
    }

    /// Set a custom pretty timestamp format for the logging part.
    fn custom_timestamp_local(io: &mut ::std::io::Write) -> ::std::io::Result<()> {
        // This is printing something close to: '2019-05-14 21:39:41.130191'
        write!(io, "{}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.6f"))
        // // This is printing something close to: '2019-05-14T21:40:29.932321271+00:00'
        // write!(io, "{}", chrono::Utc::now().format("%+"))
    }

    /// Initializes our log facility by setting it as async with the correct timestamp format.
    fn init_log() -> slog::Logger {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator)
            .use_custom_timestamp(Logger::custom_timestamp_local)
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog_o!())
    }

    /// Dump all our asynchronous logging to a file.
    fn init_log_to_file(log_file: &str) -> slog::Logger {

        // Using PathBuf here should prevent trouble between Windows & Unix
        // file path handling.
        let log_file_path = PathBuf::from(log_file);
        let log_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&log_file_path)
                .expect(&format!("Fail to create/open log file: '{:?}", &log_file_path));
        let decorator = slog_term::PlainSyncDecorator::new(log_file);
        let drain = slog_term::FullFormat::new(decorator)
            .use_custom_timestamp(Logger::custom_timestamp_local)
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog_o!())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::init()
    }
}

/// In order to stick with 'idiomatic Rust' we ship all the necessary log macros in a 'prelude'
pub mod prelude {
    #[doc(no_inline)]
    pub use {
        crate::crit, crate::debug, crate::error, crate::info, crate::slog_crit, crate::slog_debug,
        crate::slog_error, crate::slog_info, crate::slog_trace, crate::slog_warn, crate::trace,
        crate::warn,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_log_test() {
        let _guard = Logger::init();

        trace!("logging a trace message");
        debug!("debug values"; "x" => 1, "y" => -1);
        info!("some interesting info"; "where" => "right here");
        warn!("be cautious!"; "why" => "you never know...");
        error!("wrong {}", "foobar"; "type" => "unknown");
        crit!("abandoning test");
    }

    #[test]
    fn log_to_file_test() {
        use std::{
            fs::OpenOptions,
            io::{Read, Seek, SeekFrom},
        };
        use tempfile::NamedTempFile;

        let dumped_log_string = "Logged sentence to a file.";

        let tmp_file = NamedTempFile::new().expect("Fail to create temporary file.");
        let tmp_file_path = tmp_file.into_temp_path();
        let tmp_file_path = tmp_file_path.to_str().expect("Fail to convert 'PathBuf' to 'String'.");

        // We need to wrap this in its own block to force flush
        {
            let _guard = Logger::init_file_logger(&tmp_file_path);
            info!("{}", dumped_log_string);
        }
        {
            let file_path = PathBuf::from(tmp_file_path);

            let mut file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(&file_path)
                .expect(&format!("Fail to open temporary file: '{:?}", &tmp_file_path));

            // Seek to start of the temporary log file
            file.seek(SeekFrom::Start(0)).unwrap();
            let mut buf = String::new();
            file.read_to_string(&mut buf).expect("Fail to read temporary file to string.");

            assert!(
                buf.len() >= dumped_log_string.len(),
                "We should have more characters in the log file than in the sentence we compare to it");
            let s = &buf[buf.len()-1-dumped_log_string.len()..buf.len()-1];
            println!(">> line = {:?}", &s);

            assert_eq!(dumped_log_string, s)
        }
    }
}
