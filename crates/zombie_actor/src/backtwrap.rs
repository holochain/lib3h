/// specify whether we want to capture backtraces, and if they should be resolved
#[derive(Debug, Clone, Copy)]
pub enum BacktwrapCaptureStrategy {
    /// don't capture... Backtwrap will contain None
    DoNotCapture,
    /// capture unresolved backtraces... you can resolve them before printing
    CaptureUnresolved,
    /// capture resolved backtraces... they will have debug symbols
    CaptureResolved,
}

use BacktwrapCaptureStrategy::*;

lazy_static! {
    static ref CAPTURE_STRATEGY: std::sync::Mutex<BacktwrapCaptureStrategy> = {
        std::sync::Mutex::new(match std::env::var("BACKTRACE_STRATEGY") {
            Ok(s) => match s.as_str() {
                "CAPTURE_RESOLVED" => CaptureResolved,
                "CAPTURE_UNRESOLVED" => CaptureUnresolved,
                _ => DoNotCapture,
            },
            _ => DoNotCapture,
        })
    };
}

/// it seems as though
/// - linux can caputure resolved backtraces w/o much overhead
/// - macOs can capture UNresolved backtraces (fn pointers but no debug symbols)
///   without much overhead
/// - windows cannot capture backtraces at all without major slowdowns
#[derive(Shrinkwrap, Debug, Clone)]
#[shrinkwrap(mutable)]
pub struct Backtwrap(pub Option<backtrace::Backtrace>);

impl Backtwrap {
    /// Capture (or doesn't capture) backtraces based on environment variable
    ///  - default - DoNotCapture
    ///  - BACKTRACE_STRATEGY=CAPTURE_RESOLVED - CaptureResolved
    ///  - BACKTRACE_STRATEGY=CAPTURE_UNRESOLVED - CaptureUnresolved
    pub fn new() -> Self {
        Self(
            match *CAPTURE_STRATEGY.lock().expect("failed to lock mutex") {
                CaptureResolved => Some(backtrace::Backtrace::new()),
                CaptureUnresolved => Some(backtrace::Backtrace::new_unresolved()),
                DoNotCapture => None,
            },
        )
    }

    /// get the current capture strategy
    pub fn get_capture_strategy() -> BacktwrapCaptureStrategy {
        *CAPTURE_STRATEGY.lock().expect("failed to lock mutex")
    }

    /// explicitly set the current capture strategy
    pub fn set_capture_strategy(strategy: BacktwrapCaptureStrategy) {
        *CAPTURE_STRATEGY.lock().expect("failed to lock mutex") = strategy;
    }
}

impl PartialEq for Backtwrap {
    fn eq(&self, other: &Backtwrap) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

impl Eq for Backtwrap {}

impl std::hash::Hash for Backtwrap {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:?}", self).hash(state);
    }
}

impl std::convert::From<backtrace::Backtrace> for Backtwrap {
    fn from(bt: backtrace::Backtrace) -> Backtwrap {
        Self(Some(bt))
    }
}

impl std::convert::From<Option<backtrace::Backtrace>> for Backtwrap {
    fn from(bt: Option<backtrace::Backtrace>) -> Backtwrap {
        Self(bt)
    }
}

impl std::convert::From<Backtwrap> for Option<backtrace::Backtrace> {
    fn from(bt: Backtwrap) -> Option<backtrace::Backtrace> {
        bt.0
    }
}
