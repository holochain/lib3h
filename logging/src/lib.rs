//! Implementation of a non-blocking logger for `lib3h`
//!
//! # Basic example:
//!
//!
//! ```
//! use holochain_logging::{prelude::*, Lib3hLogger};
//!
//! fn main() {
//!     // Default level is `Info`, so it will skip everything lower
//!     let _guard = Lib3hLogger::init();
//!
//!     trace!("logging a trace message");
//!     debug!("debug values = {}", 42);
//!     info!("some interesting info");
//!     warn!("be cautious!");
//!     error!("wrong {} here", "stuff");
//! }
//! ```
//!
//! # Initialize from `TOML`:
//!
//! ```
//! use holochain_logging::{prelude::*, Lib3hLogger};
//!
//! fn main() {
//!     let toml = r#"
//!         [logger]
//!         level = "debug"
//!     "#;
//!
//!     let _guard = Lib3hLogger::init_log_from_toml(&toml);
//!     debug!("Setting up logging from `TOML`");
//! }
//! ```

use chrono;
use fern;
pub use log::{self, debug, error, info, trace, warn, LevelFilter};

use fern::colors::{Color, ColoredLevelConfig};
use serde_derive::Deserialize;
use std::{
    sync::mpsc::{channel, Sender},
    thread,
};
use toml;

const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;
const DEFAULT_LOG_LEVEL_STR: &'static str = "info";

/// This struct is used as a wrapper around the `slog` crate for our customized logger.
///
/// This has to be instantiate at the top of the user program in order to keep
/// the logging thread alive.
///
pub struct Lib3hLogger {
    /// Logging level: Trace, Debug, Info, Warn, Error
    level: LevelFilter,
}

#[derive(Deserialize, Debug)]
struct Lib3hLoggerConfig {
    logger: Option<Lib3hLoggerConfigLevel>,
}

#[derive(Deserialize, Debug)]
struct Lib3hLoggerConfigLevel {
    level: Option<String>,
}

impl Lib3hLogger {
    /// Initializes the Logger with the default logging level: `Info`.
    pub fn init() -> Result<Self, log::SetLoggerError> {
        Lib3hLogger::init_terminal_logger(DEFAULT_LOG_LEVEL)
    }

    /// Initializes the logger with a custom level: `Trace`, `Debug`, `Info`, `Warning` or `Error`.
    pub fn init_with_level(level: LevelFilter) -> Result<Self, log::SetLoggerError> {
        Lib3hLogger::init_terminal_logger(level)
    }

    /// Initializes our log facility by setting it as async with the correct timestamp format.
    fn init_terminal_logger(level: LevelFilter) -> Result<Self, log::SetLoggerError> {
        // Init the separated thread doing all the logging work
        let tx = Lib3hLogger::init_logging_thread();

        // Let's init le logging dispatcher
        Lib3hLogger::new_dispatch()
            .level(level)
            // .level_for("doodle_log_fern::utils", log::LevelFilter::Warn)
            .chain(tx)
            .apply()?;

        Ok(Self { level })
    }

    /// Set our logging format as a `fern::Dispatch`.
    fn new_dispatch() -> fern::Dispatch {
        let colors = Lib3hLogger::default_colors();
        fern::Dispatch::new().format(move |out, message, record| {
            out.finish(format_args!(
                "{date} | \x1B[01m{target};{line:<4}\x1B[0m | {level:<5} | {msg}",
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                target = record.target(),
                line = record.line().unwrap_or(0),
                level = colors.color(record.level()),
                msg = message,
            ))
        })
    }

    /// Ignites a thread dedicated to the logging work.
    fn init_logging_thread() -> Sender<String> {
        // Send the logging work in a separeted thread in order to achive
        // non-blocking capability
        let (tx, rx) = channel();
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                // // We can use this trick to send a signal to the logging thread to
                // // gracefully shutdown. Ideally, we would like to send some signal
                // // as 'enum' for performance & safety reason. But for now we will
                // // chose the simplest solution.
                // let msg = String::from(msg);
                // if msg.contains("Terminate Logging.") { break; }
                // else { eprint!("{}", msg); }

                eprint!("{}", msg);
            }
        });
        tx
    }

    /// Set our logging format as a `fern::Dispatch`.
    fn _new_dispatch_with_colors(colors: ColoredLevelConfig) -> fern::Dispatch {
        fern::Dispatch::new().format(move |out, message, record| {
            out.finish(format_args!(
                "{date} | \x1B[01m{target};{line:<4}\x1B[0m | {level:<5} | {msg}",
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f"),
                target = record.target(),
                line = record.line().unwrap_or(0),
                level = colors.color(record.level()),
                msg = message
            ))
        })
    }

    /// Dump all our non-blocking logging to a file.
    pub fn init_file_logger(log_file: &str) -> Result<Self, log::SetLoggerError> {
        let tx = Lib3hLogger::init_logging_thread();

        // Using PathBuf here should prevent some trouble between Windows & Unix
        // file path handling.
        let log_file_path = std::path::PathBuf::from(&log_file)
            .to_str()
            .expect("Fail to get the path of the log file.")
            .to_owned();
        Lib3hLogger::new_dispatch()
            .chain(std::io::stdout())
            .chain(
                fern::log_file(&log_file_path.clone())
                    .unwrap_or_else(|_| panic!("Fail to dump log in '{}'", &log_file_path)),
            )
            .chain(tx)
            .apply()?;
        Ok(Self {
            level: LevelFilter::Info,
        })
    }

    /// Init logging factory from a `TOML` config file.
    // fn init_log_from_toml(toml_str: &str) -> Result<Self, log::SetLoggerError> {
    pub fn init_log_from_toml(toml_str: &str) -> Result<Self, log::SetLoggerError> {
        let config: Lib3hLoggerConfig = toml::from_str(toml_str).expect("Fail to parse `TOML`.");

        let level_config = match config.logger {
            Some(config_level) => {
                // println!("cl = {:#?}", config_level);
                match config_level.level {
                    Some(v) => v.to_owned(),
                    None => String::from(DEFAULT_LOG_LEVEL_STR),
                }
            }
            None => String::from(DEFAULT_LOG_LEVEL_STR),
        };

        let level = match level_config.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warning" | "warn" => LevelFilter::Warn,
            "error" | "err" => LevelFilter::Error,
            v => {
                // Shall we panic here?
                // panic!("Unknown logging level: '{}'", v);
                println!("Unknown logging level: '{}'", v);
                LevelFilter::Info
            }
        };

        Lib3hLogger::init_terminal_logger(level);
        Ok(Self { level })
    }

    /// Returns the default colors of the logging levels.
    pub fn default_colors() -> ColoredLevelConfig {
        ColoredLevelConfig::new()
            .trace(Color::Magenta)
            .debug(Color::Green)
            .info(Color::Cyan)
            .warn(Color::Yellow)
            .error(Color::Red)
    }

    /// Returns the our current logging level.
    pub fn level(&self) -> LevelFilter {
        self.level
    }
}

/// Custom drop impl so we can give the working thread 10ms to finish cleanly.
impl Drop for Lib3hLogger {
    fn drop(&mut self) {
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

impl Default for Lib3hLogger {
    fn default() -> Self {
        Self {
            level: DEFAULT_LOG_LEVEL,
        }
    }
}

/// In order to stick with 'idiomatic Rust' we ship all the necessary log macros in a 'prelude'
pub mod prelude {
    #[doc(no_inline)]
    pub use {crate::debug, crate::error, crate::info, crate::trace, crate::warn};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_log_test() {
        let _guard = Lib3hLogger::init().expect("Fail to Initialize the logger.");

        trace!("logging a trace message");
        debug!("debug values");
        info!("some interesting info");
        warn!("be cautious!");
        error!("wrong {}", "stuff");
    }

    /// This test is tricky because of the use of a separate thread that figure it out on its own when to
    /// flush IO.
    #[test]
    fn log_to_file_test() {
        use tempfile::NamedTempFile;

        let dumped_log_string = "Logged sentence to a file.";

        let tmp_file = NamedTempFile::new().expect("Fail to create temporary file.");
        let tmp_file_path = tmp_file.into_temp_path();
        let tmp_file_path = tmp_file_path
            .to_str()
            .expect("Fail to convert 'PathBuf' to 'String'.");

        let _guard = Lib3hLogger::init_file_logger(&tmp_file_path);
        info!("{}", dumped_log_string);

        // // This is the messed up part...
        // {
        //     use std::{
        //         fs::OpenOptions,
        //         io::{Read, Seek, SeekFrom},
        //         path::PathBuf,
        //     };
        //     let file_path = PathBuf::from(tmp_file_path);
        //
        //     let mut file = OpenOptions::new()
        //         .read(true)
        //         .write(false)
        //         .create(false)
        //         .open(&file_path)
        //         .expect(&format!(
        //                 "Fail to open temporary file: '{:?}",
        //                 &tmp_file_path
        //         ));
        //
        //     // Seek to start of the temporary log file
        //     file.seek(SeekFrom::Start(0)).unwrap();
        //     let mut buf = String::new();
        //     file.read_to_string(&mut buf)
        //         .expect("Fail to read temporary file to string.");
        //
        //     assert!(
        //         buf.len() >= dumped_log_string.len(),
        //         "We should have more characters in the log file than in the sentence we compare to it");
        //     let s = (&buf[buf.len() - 1 - dumped_log_string.len()..buf.len()]).trim();
        //
        //     assert_eq!(dumped_log_string, s)
        // }
    }

    #[test]
    fn log_from_toml_test() {
        let toml = r#"
        [[agents]]
        id = "test agent"
        name = "Holo Tester 1"
        public_address = "HoloTester1-------------------------------------------------------------------------AHi1"
        keystore_file = "holo_tester.key"

        [[dnas]]
        id = "app spec rust"
        file = "app_spec.dna.json"
        hash = "Qm328wyq38924y"

        [[instances]]
        id = "app spec instance"
        dna = "app spec rust"
        agent = "test agent"
            [instances.storage]
            type = "file"
            path = "app_spec_storage"

        [[interfaces]]
        id = "app spec websocket interface"
            [interfaces.driver]
            type = "websocket"
            port = 8888
            [[interfaces.instances]]
            id = "app spec instance"

        [logger]
        level = "debug"
            [[logger.rules.rules]]
            pattern = ".*"
            color = "red"
        "#;

        let _guard = Lib3hLogger::init_log_from_toml(&toml);
        debug!("Logging set up from `TOML`!");
    }
}
