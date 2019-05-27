//! Implementation of an non-blocking logger for `lib3h`

// #[macro_use]
// extern crate log;
use chrono;
use fern;
pub use log::{self, debug, error, info, trace, warn};

use fern::colors::{Color, ColoredLevelConfig};
use std::sync::mpsc::{channel, Sender};
use std::thread;

// TODO: Add recv to guard struct in order to close the working thread <24-05-19, yourname> //

/// This struct is used as a wrapper around the `slog` crate for our customized logger.
///
/// This has to be instantiate at the top of the user program in order to keep
/// the logging thread alive.

pub struct Lib3hLogger;

impl Lib3hLogger {
    pub fn init() -> Result<Self, log::SetLoggerError> {
        Lib3hLogger::init_terminal_logger()
    }

    /// Log directly in the terminal.
    pub fn init_terminal_logger() -> Result<Self, log::SetLoggerError> {
        let log = Lib3hLogger::init_log()?;
        Ok(log)
    }

    /// Initializes our log facility by setting it as async with the correct timestamp format.
    fn init_log() -> Result<Self, log::SetLoggerError> {
        let tx = Lib3hLogger::init_logging_thread();
        Lib3hLogger::new_dispatch()
            .chain(tx)
            // .level(log::LevelFilter::Info)
            // .level_for("doodle_log_fern::utils", log::LevelFilter::Warn)
            .apply()?;
        Ok(Self)
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
                msg = message
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
            // .expect(&format!("Fail to dump log in '{}'", &log_file_path)))
            .chain(tx)
            .apply()?;
        Ok(Self)
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
}

/// Custom drop impl so we can give the working thread 10ms to finish cleanly.
impl Drop for Lib3hLogger {
    fn drop(&mut self) {
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

impl Default for Lib3hLogger {
    fn default() -> Self {
        Lib3hLogger::init_terminal_logger()
            .expect("Fail to Initialize the Logger with default configuration.")
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

    // #[test]
    // fn init_log_test() {
    //     let _guard = Lib3hLogger::init().expect("Fail to Initialize the logger.");
    //
    //     trace!("logging a trace message");
    //     debug!("debug values");
    //     info!("some interesting info");
    //     warn!("be cautious!");
    //     error!("wrong {}", "stuff");
    // }

    /// This test is tricky because of the use of a separate thread that figure on its own when to
    /// flush IO.
    #[test]
    fn log_to_file_test() {
        use std::{
            fs::OpenOptions,
            io::{Read, Seek, SeekFrom},
            path::PathBuf,
        };
        use tempfile::NamedTempFile;

        let dumped_log_string = "Logged sentence to a file.";

        let tmp_file = NamedTempFile::new().expect("Fail to create temporary file.");
        let tmp_file_path = tmp_file.into_temp_path();
        let tmp_file_path = tmp_file_path
            .to_str()
            .expect("Fail to convert 'PathBuf' to 'String'.");

        // We need to wrap this in its own block to force flush
        {
            let _guard = Lib3hLogger::init_file_logger(&tmp_file_path);
            info!("{}", dumped_log_string);
        }
        {
            let file_path = PathBuf::from(tmp_file_path);

            let mut file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(&file_path)
                .expect(&format!(
                    "Fail to open temporary file: '{:?}",
                    &tmp_file_path
                ));

            // Seek to start of the temporary log file
            file.seek(SeekFrom::Start(0)).unwrap();
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("Fail to read temporary file to string.");

            assert!(
                buf.len() >= dumped_log_string.len(),
                "We should have more characters in the log file than in the sentence we compare to it");
            let s = (&buf[buf.len() - 1 - dumped_log_string.len()..buf.len()]).trim();

            assert_eq!(dumped_log_string, s)
        }
    }
}
