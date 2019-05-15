use holochain_logging::{prelude::*, Logger};

pub fn main() {
    let _guard = Logger::new();

    trace!("logging a trace message");
    debug!("debug values"; "x" => 1, "y" => -1);
    info!("some interesting info"; "where" => "right here");
    warn!("be cautious!"; "why" => "you never know...");
    error!("wrong {}", "foobar"; "type" => "unknown");
    crit!("abandoning test");
}
