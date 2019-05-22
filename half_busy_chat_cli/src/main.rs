//! HalfBusyChat Command Line Utility / Manual Testing CLI

extern crate lib3h_half_busy_chat;
extern crate linefeed;

fn main() {
    let rl = std::sync::Arc::new(
        linefeed::Interface::new("half_busy_chat_cli").expect("failed to init linefeed"),
    );

    rl.set_report_signal(linefeed::terminal::Signal::Interrupt, true);
    rl.set_ignore_signal(linefeed::terminal::Signal::Interrupt, false);
    rl.set_prompt("busychat> ")
        .expect("failed to set linefeed prompt");

    let rl_t = rl.clone();
    let mut cli = lib3h_half_busy_chat::HalfBusyChat::new(Box::new(move |event| {
        writeln!(rl_t, "GOT {:?}", event).expect("write fail");
    }));

    loop {
        let res = rl.read_line();
        match res {
            Ok(line) => match line {
                linefeed::reader::ReadResult::Input(s) => {
                    cli.send(lib3h_half_busy_chat::ChatEvent::Message(
                        lib3h_half_busy_chat::MessageData {
                            from_address: "[null]".to_string(),
                            payload: s,
                        },
                    ));
                }
                linefeed::reader::ReadResult::Eof => {
                    eprintln!("input read fail Eof");
                    break;
                }
                linefeed::reader::ReadResult::Signal(s) => {
                    eprintln!("input read fail {:?}", s);
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
