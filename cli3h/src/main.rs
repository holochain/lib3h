extern crate lib3h;
extern crate toml;

static CONFIG_EXAMPLE: &'static str = r#"
# identity_type for now is only 'Ephemeral'
# someday we will support a 'Supplied' type
identity_type = 'Ephemeral'

# list all the network interfaces you want to bind to
# note for now only the first is used
# (but you can specify '0.0.0.0')
[[binding_endpoints]]
addr = '0.0.0.0'
port = 8081

# list all the bootstrap nodes you want to try connecting to

# bootstrap 1
[[bootstrap_endpoints]]
addr = '127.0.0.1'
port = 8080

# bootstrap 2
[[bootstrap_endpoints]]
addr = '127.0.0.1'
port = 8082
"#;

use lib3h::config::{IdentityType, NodeConfig};
use lib3h::error;
use lib3h::netinfo::Endpoint;
use lib3h::node::{Node, NodeEvent};
use lib3h::tcp;

fn run() -> error::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let prog = match std::path::Path::new(&args[0]).file_name() {
        Some(v) => String::from(v.to_string_lossy()),
        None => String::from("p2hackcli"),
    };

    let usage = move || {
        eprintln!(
            r#"usage: {} [configfile]
usage: {} > config.toml
"#,
            prog, prog
        );
        println!("{}", CONFIG_EXAMPLE);
    };

    if args.len() != 2 {
        usage();
        return Err(error::Error::from("config not found"));
    }

    let config = args[1].to_string();
    let config = std::path::Path::new(&config);
    let config = std::fs::read(config)?;
    let config = String::from_utf8_lossy(&config);
    let config: NodeConfig = toml::from_str(&config).unwrap();

    let mut node: Node<tcp::StdNetServer> = Node::new(config)?;

    loop {
        let events;

        {
            events = node.process_once();
        }

        if events.len() < 1 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        for event in events {
            match event {
                NodeEvent::OnWait => (),
                NodeEvent::OnError(e) => {
                    eprintln!("{:?}", e);
                }
                NodeEvent::OnIncomingRequest(r) => {
                    println!("Got request: {:?}", r);
                }
                NodeEvent::OnOutgoingResponse(r) => {
                    println!("Got response: {:?}", r);
                }
            }
        }
    }

    Ok(())
}

fn main() {
    match run() {
        Err(err) => {
            eprintln!("error: {:?}", err);
            std::process::exit(1);
        }
        _ => (),
    }
}
