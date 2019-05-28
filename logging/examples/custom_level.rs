use holochain_logging::{prelude::*, Lib3hLogger};
use log::LevelFilter;

fn main() {
    // let _ = Lib3hLogger::init().unwrap();
    // let _guard = Lib3hLogger::init_with_level(LevelFilter::Info).unwrap();
    let _guard = Lib3hLogger::init_with_level(LevelFilter::Debug).unwrap();

    info!("Some info here");
    debug!("Logging set up from `TOML`!");
}
