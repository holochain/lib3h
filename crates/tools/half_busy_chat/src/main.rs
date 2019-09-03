//! HalfBusyChat Command Line Utility / Manual Testing CLI

extern crate linefeed;
use lib3h_half_busy_chat::{ChatEvent, channel_address_from_str};

fn main() {
    let rl = std::sync::Arc::new(
        linefeed::Interface::new("half_busy_chat").expect("failed to init linefeed"),
    );

    rl.set_report_signal(linefeed::terminal::Signal::Interrupt, true);
    rl.set_prompt("busychat> ")
        .expect("failed to set linefeed prompt");

    let rl_t = rl.clone();
    let mut cli = lib3h_half_busy_chat::HalfBusyChat::new(Box::new(move |event| {
        writeln!(rl_t, "GOT {:?}", event).expect("write fail");
    }));

    let help_text = || {
        writeln!(
            rl,
            r#"
Half Busy Chat Commands:
  /help         - this help text
  /quit         - exit Half Busy Chat
"#
        )
        .expect("write fail");
    };

    help_text();

    loop {
        let res = rl.read_line();
        match res {
            Ok(line) => match line {
                linefeed::reader::ReadResult::Input(s) => {
                    if s.starts_with("/quit") {
                        writeln!(rl, "QUIT").expect("write fail");
                        return;
                    } else if s.starts_with("/help") {
                        help_text();
                    } else if s.starts_with("/join ") {
                        let channel_id = &s["/join ".len()..];
                        let channel_addr = channel_address_from_str(channel_id).expect("failed to hash channel address");
                        cli.send(ChatEvent::Join(channel_addr))
                    } else {
                        cli.send(ChatEvent::Message(
                            lib3h_half_busy_chat::MessageData {
                                from_address: "[null]".to_string(),
                                payload: s,
                            },
                        ));
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
