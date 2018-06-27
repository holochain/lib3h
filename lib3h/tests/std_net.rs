extern crate lib3h;
extern crate rand;

use lib3h::tcp::{Connection, Server};
use rand::Rng;

static LOCAL: &'static str = "127.0.0.1";

fn _help_find_listen_socket() -> (lib3h::tcp::StdNetServer, u16) {
    let mut rng = rand::thread_rng();

    let mut port: u16 = 10000 + rng.gen_range(10_u16, 500_u16);
    let mut loop_count = 0;

    'outer: loop {
        let mut srv = lib3h::tcp::StdNetServer::new();
        srv.listen(LOCAL, port);
        loop {
            let events = srv.process_once();
            if events.len() < 1 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            for event in events {
                match event {
                    lib3h::tcp::Event::OnConnected => {
                        return (srv, port);
                    }
                    _ => {
                        loop_count += 1;
                        if loop_count > 10 {
                            panic!("failed to find listening port");
                        }
                        port = port + rng.gen_range(1_u16, 10_u16);
                        continue 'outer;
                    }
                }
            }
        }
    }
}

fn _help_get_con(srv: &mut lib3h::tcp::StdNetServer) -> Box<lib3h::tcp::Connection> {
    loop {
        let events = srv.process_once();
        if events.len() < 1 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }
        for event in events {
            match event {
                lib3h::tcp::Event::OnConnection(con) => {
                    return con;
                }
                _ => panic!("bad event"),
            }
        }
    }
}

fn _help_get_data(con: &mut Box<lib3h::tcp::Connection>) -> String {
    loop {
        let events = con.process_once();
        if events.len() < 1 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }
        for event in events {
            match event {
                lib3h::tcp::Event::OnData(data) => {
                    return String::from_utf8_lossy(&data).to_string();
                }
                _ => panic!("bad event"),
            }
        }
    }
}

#[test]
fn it_std_net_full() {
    let (mut srv, port) = _help_find_listen_socket();

    let mut con: Box<lib3h::tcp::Connection> =
        Box::new(lib3h::tcp::StdNetConnection::new());
    con.connect(LOCAL, port);
    con.process_once(); // drain events

    let mut srv_con = _help_get_con(&mut srv);

    con.send(b"hello");

    assert_eq!(_help_get_data(&mut srv_con), String::from("hello"));

    srv_con.send(b"hello2");

    assert_eq!(_help_get_data(&mut con), String::from("hello2"));
}
