//! Implementation of an asynchronous logger for `lib3h`

use chrono;
pub use slog_async;
pub use slog_scope::{crit, debug, error, info, trace, warn};
pub use slog_term;

use slog::Drain;
pub use slog::{slog_crit, slog_debug, slog_error, slog_info, slog_o, slog_trace, slog_warn};

// pub struct Logger {
//     _global_logger_guard: slog_scope::GlobalLoggerGuard,
// }
/// This struct is used as a wrapper around the `slog` crate for our customized logger.
/// This has to be instantiate at the top of the user program in order to keep
/// the logging thread alive.
pub struct Logger(slog_scope::GlobalLoggerGuard);

impl Logger {
    pub fn init() -> Self {
        Logger::init_terminal_logger()
    }

    /// Log directly in the terminal.
    fn init_terminal_logger() -> Self {
        Self(slog_scope::set_global_logger(Logger::init_log()))
    }

    /// Set a custom pretty timestamp format for the logging part.
    fn custom_timestamp_local(io: &mut ::std::io::Write) -> ::std::io::Result<()> {
        // This is printing something close to: '2019-05-14 21:39:41.130191'
        write!(io, "{}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.6f"))
        // // This is printing something close to: '2019-05-14T21:40:29.932321271+00:00'
        // write!(io, "{}", chrono::Utc::now().format("%+"))
    }

    /// Initialises our log facility by setting it as async with the correct timestamp format.
    fn init_log() -> slog::Logger {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator)
        // let drain = slog_term::FullFormat::new(decorator)
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
    // #[doc(no_inline)] pub use crate::Logger;
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
}
