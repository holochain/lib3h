//! SimChat Command Line Utility / Manual Testing CLI

mod lib3h_simchat;
mod simchat;

extern crate chrono;
extern crate colored;
extern crate linefeed;
extern crate regex;
extern crate url;

use crate::simchat::{ChatEvent, SimChat, SimChatMessage};
use chrono::prelude::DateTime;
use colored::*;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{EngineConfig, GhostEngine, TransportConfig},
};
use lib3h_sodium::SodiumCryptoSystem;
use regex::Regex;
use std::path::PathBuf;
use url::Url;

use std::time::{Duration, UNIX_EPOCH};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "Lib3h SimChat", about = "A p2p, IRC style chat client")]
struct Opt {
    /// URLs of nodes to use as bootstraps
    #[structopt(short = "b", long)]
    bootstrap_nodes: Vec<Url>,
}
use lib3h_tracing::Lib3hSpan;

fn engine_builder() -> GhostEngine<'static> {
    let crypto = Box::new(SodiumCryptoSystem::new());
    let config = EngineConfig {
        transport_configs: vec![TransportConfig::Memory],
        bootstrap_nodes: vec![],
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let dht_factory = MirrorDht::new_with_config;
    GhostEngine::new(
        Lib3hSpan::fixme(), // TODO: actually hook up real tracer here
        crypto,
        config,
        "test_engine",
        dht_factory,
    )
    .unwrap()
}

fn main() {
    let opt = Opt::from_args();

    let rl =
        std::sync::Arc::new(linefeed::Interface::new("sim_chat").expect("failed to init linefeed"));

    rl.set_report_signal(linefeed::terminal::Signal::Interrupt, true);
    rl.set_prompt("connecting...> ")
        .expect("failed to set linefeed prompt");

    let rl_t = rl.clone();
    let mut cli = lib3h_simchat::Lib3hSimChat::new(
        engine_builder,
        Box::new(move |event| {
            match event {
                ChatEvent::JoinSuccess { channel_id, .. } => {
                    rl_t.set_prompt(&format!("#{}> ", channel_id).to_string())
                        .expect("failed to set linefeed prompt");
                }
                ChatEvent::PartSuccess { .. } => {
                    rl_t.set_prompt("SimChat> ")
                        .expect("failed to set linefeed prompt");
                }
                ChatEvent::ReceiveDirectMessage(SimChatMessage {
                    from_agent,
                    payload,
                    timestamp,
                }) => {
                    writeln!(
                        rl_t,
                        "{} | *{}* {}",
                        format_timestamp(*timestamp).white().dimmed(),
                        from_agent.yellow(),
                        payload.blue()
                    )
                    .expect("write fail");
                }
                ChatEvent::ReceiveChannelMessage(SimChatMessage {
                    from_agent,
                    payload,
                    timestamp,
                }) => {
                    writeln!(
                        rl_t,
                        "{} | {}: {}",
                        format_timestamp(*timestamp).white().dimmed(),
                        from_agent.yellow(),
                        payload.blue()
                    )
                    .expect("write fail");
                }
                ChatEvent::Connected => {
                    rl_t.set_prompt("SimChat> ")
                        .expect("failed to set linefeed prompt");
                }
                ChatEvent::Disconnected => {
                    rl_t.set_prompt("connecting...> ")
                        .expect("failed to set linefeed prompt");
                }
                _ => {}
            }
            // writeln!(rl_t, "SIMCHAT GOT {:?}", event).expect("write fail");
        }),
        opt.bootstrap_nodes,
    );

    let help_text = || {
        writeln!(
            rl,
            r#"
lib3h simchat Commands:
  /help                     - this help text
  /join <space> <handle>    - Join a space assigning yourself a handle
  /part <space>             - Leave a given space
  /msg <agent> <msg>        - Send a direct message to an agent in your space
  /bootstrap <peer_uri>     - Attempt to bootstrap onto the network via a given peer uri
  /quit                     - exit Sim Chat
"#
        )
        .expect("write fail");
    };

    help_text();

    // matches commands beginnign with / and captures the command name and the args
    let command_matcher = Regex::new(r"^/([a-z]+)\s?(.*)$").expect("This is a valid regex");

    loop {
        let res = rl.read_line_step(Some(Duration::from_millis(1000)));
        match res {
            Ok(Some(line)) => {
                match line {
                    linefeed::reader::ReadResult::Input(s) => {
                        if s.starts_with('/') {
                            let caps = command_matcher.captures(&s).expect("capture failed");
                            let command = caps.get(1).map(|s| s.as_str());
                            let args = caps.get(2).map(|s| s.as_str());
                            match (command, args) {
                                (Some("quit"), _) => {
                                    writeln!(rl, "QUIT").expect("write fail");
                                    return;
                                }
                                (Some("help"), _) => {
                                    help_text();
                                }
                                (Some("join"), Some(rest)) => {
                                    let mut words = rest.split(' ');
                                    let channel_id = words.next();
                                    let agent_id = words.next();
                                    if let (Some(channel_id), Some(agent_id)) =
                                        (channel_id, agent_id)
                                    {
                                        cli.send(ChatEvent::Join {
                                            channel_id: channel_id.to_string(),
                                            agent_id: agent_id.to_string(),
                                        })
                                    } else {
                                        writeln!(rl, "/join must be called with two args, a channel_id and an agent_id").expect("write fail");
                                    }
                                }
                                (Some("part"), Some(channel_id)) => cli.send(ChatEvent::Part {
                                    channel_id: channel_id.to_string(),
                                }),
                                (Some("msg"), Some(rest)) => {
                                    let mut words = rest.split(' ');
                                    let to_agent: String = words.next().unwrap().to_string();
                                    let payload: String = words.collect();
                                    cli.send(ChatEvent::SendDirectMessage { to_agent, payload });
                                }
                                (Some("bootstrap"), Some(uri_str)) => {
                                    if let Ok(uri) = Url::parse(uri_str) {
                                        cli.send(ChatEvent::Bootstrap(uri));
                                    } else {
                                        writeln!(rl, "/bootstrap must be called with a valid URI as a parameter").expect("write fail");
                                    }
                                }
                                _ => {
                                    writeln!(
                                        rl,
                                        "Unrecognised command or arguments not correctly given"
                                    )
                                    .expect("write fail");
                                }
                            }
                        } else {
                            if s.len() > 0 {
                                // no sending empty messages
                                cli.send(ChatEvent::SendChannelMessage { payload: s });
                            }
                        }
                    }
                    linefeed::reader::ReadResult::Eof => {
                        eprintln!("\nEof");
                        break;
                    }
                    linefeed::reader::ReadResult::Signal(s) => {
                        eprintln!("\nSignal: {:?}", s);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
                break;
            }
            Ok(None) => {} // keep waiting for input
        }

        // currently there is no concept of time anchors so this will just poll every message in the space
        // In future this will calculate the correct time anchors and filter the results by timestamp
        cli.send(ChatEvent::QueryChannelMessages {
            start_time: 0,
            end_time: 0,
        });

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let d = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = DateTime::<chrono::Utc>::from(d);
    datetime.format("%H:%M:%S").to_string()
}
