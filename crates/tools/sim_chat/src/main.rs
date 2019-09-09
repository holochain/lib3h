//! SimChat Command Line Utility / Manual Testing CLI

extern crate linefeed;
extern crate regex;
extern crate url;
use lib3h_sim_chat::{ChatEvent};
use regex::Regex;
use url::Url;

fn main() {
    let rl =
        std::sync::Arc::new(linefeed::Interface::new("sim_chat").expect("failed to init linefeed"));

    rl.set_report_signal(linefeed::terminal::Signal::Interrupt, true);
    rl.set_prompt("simchat> ")
        .expect("failed to set linefeed prompt");

    let rl_t = rl.clone();
    let mut cli = lib3h_sim_chat::SimChat::new(
        Box::new(move |event| {
            writeln!(rl_t, "GOT {:?}", event).expect("write fail");
        }),
        Url::parse("http://bootstrap.holo.host").unwrap()
    );

    let help_text = || {
        writeln!(
            rl,
            r#"
lib3h simchat Commands:
  /help                     - this help text
  /join <space> <handle>    - Join a space assigning yourself a handle
  /part                     - Leave the current space
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
        let res = rl.read_line();
        match res {
            Ok(line) => match line {
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
                                    cli.send(ChatEvent::Join{channel_id: channel_id.to_string(), agent_id: agent_id.to_string()})
                                } else {
                                    writeln!(rl, "/join must be called with two args, a channel_id and an agent_id").expect("write fail");
                                }
                            }
                            (Some("part"), Some(channel_id)) => {
                                cli.send(ChatEvent::Part{channel_id:channel_id.to_string()})
                            }
                            (Some("msg"), Some(rest)) => {
                                let mut words = rest.split(' ');
                                let to_address: String = words.next().unwrap().to_string();
                                let payload: String = words.collect();
                                cli.send(ChatEvent::SendDirectMessage {
                                    to_address,
                                    payload,
                                });
                            }
                            _ => {
                                println!("Unrecognised command or arguments not correctly given");
                            }
                        }
                    } else {
                        writeln!(rl, "UNIMPLEMENTD - Cannot send non-direct messages yet")
                            .expect("write fail");
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
            }
        }
    }
}
