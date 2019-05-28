use holochain_logging::{prelude::*, Lib3hLogger};

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

fn main() {
    let _guard = Lib3hLogger::init();

    trace!("logging a trace message");
    debug!("debug values = {}", 42);
    info!("some interesting info");
    warn!("be cautious!");
    error!("wrong {} here", "stuff");

    log_from_toml_test();
}
