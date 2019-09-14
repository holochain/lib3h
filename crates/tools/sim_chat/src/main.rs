//! SimChat Command Line Utility / Manual Testing CLI

mod lib3h_simchat;
mod simchat;

extern crate linefeed;
extern crate regex;
extern crate url;
extern crate chrono;
use std::time::UNIX_EPOCH;
use chrono::prelude::DateTime;
use crate::simchat::{ChatEvent, SimChat, SimChatMessage};
use regex::Regex;
use std::thread;
use url::Url;
use std::time::Duration;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{ghost_engine::GhostEngine, RealEngineConfig},
};
use lib3h_sodium::SodiumCryptoSystem;
use lib3h_tracing::{Lib3hSpan, Reporter, span::SpanReceiver, Tracer, AllSampler};

fn engine_builder() -> GhostEngine<'static> {
    let crypto = Box::new(SodiumCryptoSystem::new());
    let config = RealEngineConfig {
        socket_type: "mem".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let dht_factory = MirrorDht::new_with_config;
    GhostEngine::new_mock(
        Lib3hSpan::fixme(), // TODO: actually hook up real tracer here
        crypto,
        config,
        "test_engine",
        dht_factory,
    )
    .unwrap()
}

/// TODO: include way to break this loop
fn run_reporter_thread(span_rx: SpanReceiver) {
    thread::spawn(move || {
        let reporter = Reporter::new("sim_chat").unwrap();
        for span in span_rx {
            reporter.report(&[span]).expect("could not report");
        }
    });
}

fn main() {
    let rl =
        std::sync::Arc::new(linefeed::Interface::new("sim_chat").expect("failed to init linefeed"));

    rl.set_report_signal(linefeed::terminal::Signal::Interrupt, true);
    rl.set_prompt("connecting...> ")
        .expect("failed to set linefeed prompt");

    let (span_tx, span_rx) = crossbeam_channel::unbounded();
    let tracer = Tracer::with_sender(AllSampler, span_tx);
    
    run_reporter_thread(span_rx);

    let rl_t = rl.clone();
    let mut cli = lib3h_simchat::Lib3hSimChat::new(
        engine_builder,
        Box::new(move |event| {
            match event {
                ChatEvent::JoinSuccess { channel_id, .. } => {
                    rl_t.set_prompt(&format!("#{}> ", channel_id).to_string())
                        .expect("failed to set linefeed prompt");
                }
                ChatEvent::PartSuccess(_) => {
                    rl_t.set_prompt("SimChat> ")
                        .expect("failed to set linefeed prompt");
                }
                ChatEvent::ReceiveDirectMessage(SimChatMessage {
                    from_agent,
                    payload,
                    timestamp,
                }) => {
                    writeln!(rl_t, "[{}] | *{}* {}", 
                        format_timestamp(timestamp),
                        from_agent,
                        payload
                    ).expect("write fail");
                },
                ChatEvent::ReceiveChannelMessage(SimChatMessage {
                    from_agent,
                    payload,
                    timestamp,
                }) => {

                    writeln!(rl_t, "[{}] | {}: {}",
                        format_timestamp(timestamp), 
                        from_agent, 
                        payload
                    ).expect("write fail");
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
        Url::parse("http://bootstrap.holo.host").unwrap(),
        tracer,
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
            Ok(Some(line)) => match line {
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
                                if let (Some(channel_id), Some(agent_id)) = (channel_id, agent_id) {
                                    cli.send(ChatEvent::Join {
                                        channel_id: channel_id.to_string(),
                                        agent_id: agent_id.to_string(),
                                    })
                                } else {
                                    writeln!(rl, "/join must be called with two args, a channel_id and an agent_id").expect("write fail");
                                }
                            }
                            (Some("part"), Some(channel_id)) => {
                                cli.send(ChatEvent::Part(channel_id.to_string()))
                            }
                            (Some("msg"), Some(rest)) => {
                                let mut words = rest.split(' ');
                                let to_agent: String = words.next().unwrap().to_string();
                                let payload: String = words.collect();
                                cli.send(ChatEvent::SendDirectMessage { to_agent, payload });
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
                        if s.len() > 0 { // no sending empty messages
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
            },
            Err(e) => {
                eprintln!("{:?}", e);
                break;
            },
            Ok(None) => {}, // keep waiting for input
        }
        cli.send(ChatEvent::QueryChannelMessages{start_time: 0, end_time: 0});
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn format_timestamp(timestamp: &u64) -> String {
    let d = UNIX_EPOCH + Duration::from_secs(*timestamp);
    let datetime = DateTime::<chrono::Utc>::from(d);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
