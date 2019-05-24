use holochain_logging::{prelude::*, Lib3hLogger};
// use holochain_logging::Lib3hLogger;

pub fn main() {
    let _guard = Lib3hLogger::init();

    trace!("logging a trace message");
    debug!("debug values = {}", 42);
    info!("some interesting info");
    warn!("be cautious!");
    error!("wrong {} here", "stuff");
}
