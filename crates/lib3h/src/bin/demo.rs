#[macro_use]
extern crate detach;

use detach::Detach;
use holochain_tracing::*;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{engine_actor::*, *},
    error::*,
    transport::websocket::tls::TlsConfig,
};
use lib3h_protocol::{data_types::*, protocol::*, uri::Lib3hUri};
use lib3h_sodium::SodiumCryptoSystem;
use lib3h_zombie_actor::*;
use url::Url;

static NET_ID: &'static str = "query-demo-network";
static SPACE_ID: &'static str = "query-demo-space";
static ENTRY_ADDR: &'static str = "query-demo-entry";
static ASPECT_ADDR: &'static str = "query-demo-aspect";
static A_1_ID: &'static str = "agent-1-id";
static A_2_ID: &'static str = "agent-2-id";

fn print_result<D: std::fmt::Debug>(r: D) {
    println!("-- begin result --\n{:#?}\n--  end result  --", r);
}

struct EngineContainer<
    E: GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    >,
> {
    engine1: Detach<GhostEngineParentWrapper<EngineContainer<E>, E, Lib3hError>>,
    #[allow(dead_code)]
    engine1_addr: Lib3hUri,
    engine2: Detach<GhostEngineParentWrapper<EngineContainer<E>, E, Lib3hError>>,
    engine2_addr: Lib3hUri,
}

impl<'lt> EngineContainer<GhostEngine<'lt>> {
    pub fn new(ws: bool) -> Self {
        let crypto = Box::new(SodiumCryptoSystem::new());

        let (transport_config, bind_url) = if ws {
            (
                TransportConfig::Websocket(TlsConfig::Unencrypted),
                Url::parse("ws://127.0.0.1:63518").unwrap().into(),
            )
        } else {
            (
                TransportConfig::Memory("send-demo".to_string()),
                Lib3hUri::with_undefined(),
            )
        };

        let mut config = EngineConfig {
            network_id: GatewayId {
                nickname: NET_ID.to_string(),
                id: NET_ID.to_string().into(),
            },
            transport_configs: vec![transport_config],
            bootstrap_nodes: vec![],
            work_dir: std::path::PathBuf::new(),
            log_level: 'd',
            bind_url: bind_url,
            dht_gossip_interval: 100,
            dht_timeout_threshold: 1000,
            dht_custom_config: vec![],
        };

        let dht_factory = MirrorDht::new_with_config;

        let e1 = GhostEngine::new(
            Span::fixme(),
            crypto.clone(),
            config.clone(),
            "send-demo-e1",
            dht_factory,
        )
        .unwrap();
        let e1_addr = e1.advertise();
        println!("e1: {}", e1_addr);

        let e1 = GhostParentWrapper::new(e1, "e1_");

        if ws {
            config.bootstrap_nodes = vec![config.bind_url.clone()];
            config.bind_url = Url::parse("wss://127.0.0.1:63519").unwrap().into();
        }

        let e2 =
            GhostEngine::new(Span::fixme(), crypto, config, "send-demo-e2", dht_factory).unwrap();
        let e2_addr = e2.advertise();
        println!("e2: {}", e2_addr);

        let e2 = GhostParentWrapper::new(e2, "e2_");

        let mut out = Self {
            engine1: Detach::new(e1),
            engine1_addr: e1_addr,
            engine2: Detach::new(e2),
            engine2_addr: e2_addr,
        };

        out.process();
        out.process();

        out.engine1
            .request(
                Span::fixme(),
                ClientToLib3h::Bootstrap(BootstrapData {
                    space_address: NET_ID.into(),
                    bootstrap_uri: out.engine2_addr.clone(),
                }),
                Box::new(|_, r| {
                    println!("1 got: {:?}", r);
                    Ok(())
                }),
            )
            .unwrap();

        out.process();
        out.process();

        out.engine1
            .request(
                Span::fixme(),
                ClientToLib3h::JoinSpace(SpaceData {
                    request_id: "".to_string(),
                    space_address: SPACE_ID.to_string().into(),
                    agent_id: A_1_ID.to_string().into(),
                }),
                Box::new(|_, r| {
                    println!("1 got: {:?}", r);
                    Ok(())
                }),
            )
            .unwrap();

        out.engine2
            .request(
                Span::fixme(),
                ClientToLib3h::JoinSpace(SpaceData {
                    request_id: "".to_string(),
                    space_address: SPACE_ID.to_string().into(),
                    agent_id: A_2_ID.to_string().into(),
                }),
                Box::new(|_, r| {
                    println!("2 got: {:?}", r);
                    Ok(())
                }),
            )
            .unwrap();

        out.process();
        out.process();

        out
    }

    fn priv_process(&mut self) {
        detach_run!(self.engine1, |e| { e.process(self) }).unwrap();
        detach_run!(self.engine2, |e| { e.process(self) }).unwrap();
        for mut msg in self.engine1.drain_messages() {
            let payload = msg.take_message();
            println!("1 got: {:?}", payload);
            match payload {
                Some(Lib3hToClient::HandleQueryEntry(q_data)) => {
                    msg.respond(Ok(Lib3hToClientResponse::HandleQueryEntryResult(
                        QueryEntryResultData {
                            space_address: SPACE_ID.to_string().into(),
                            entry_address: ENTRY_ADDR.to_string().into(),
                            request_id: "TEST_REQ_ID".to_string(),
                            requester_agent_id: A_1_ID.to_string().into(),
                            responder_agent_id: A_1_ID.to_string().into(),
                            query_result: format!(
                                "echo: {}",
                                String::from_utf8_lossy(&q_data.query)
                            )
                            .into_bytes()
                            .into(),
                        },
                    )))
                    .unwrap();
                }
                Some(Lib3hToClient::HandleStoreEntryAspect(a_data)) => print_result(a_data),
                _ => (),
            }
        }
        for mut msg in self.engine2.drain_messages() {
            let payload = msg.take_message();
            println!("2 got: {:?}", payload);
            match payload {
                Some(Lib3hToClient::HandleSendDirectMessage(dm_data)) => {
                    msg.respond(Ok(Lib3hToClientResponse::HandleSendDirectMessageResult(
                        DirectMessageData {
                            space_address: dm_data.space_address,
                            request_id: dm_data.request_id,
                            to_agent_id: dm_data.from_agent_id,
                            from_agent_id: dm_data.to_agent_id,
                            content: format!("echo: {}", String::from_utf8_lossy(&dm_data.content))
                                .into_bytes()
                                .into(),
                        },
                    )))
                    .unwrap();
                }
                Some(Lib3hToClient::HandleStoreEntryAspect(a_data)) => print_result(a_data),
                _ => (),
            }
        }
    }

    pub fn process(&mut self) {
        self.priv_process();
        // send retry interval is 20ms... this gives us ~50ms, so 2.5 attempts?
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.priv_process();
        }
    }

    pub fn send_1_to_2(&mut self) {
        self.engine1
            .request(
                Span::fixme(),
                ClientToLib3h::SendDirectMessage(DirectMessageData {
                    space_address: SPACE_ID.to_string().into(),
                    request_id: "TEST_REQ_ID".to_string(),
                    to_agent_id: A_2_ID.to_string().into(),
                    from_agent_id: A_1_ID.to_string().into(),
                    content: b"bob".to_vec().into(),
                }),
                Box::new(|_, r| {
                    print_result(r);
                    Ok(())
                }),
            )
            .unwrap();
        self.process();
        // send needs extra process calls
        self.process();
        self.process();
        self.process();
        self.process();
        self.process();
    }

    pub fn query_1(&mut self) {
        self.engine1
            .request(
                Span::fixme(),
                ClientToLib3h::QueryEntry(QueryEntryData {
                    space_address: SPACE_ID.to_string().into(),
                    entry_address: ENTRY_ADDR.to_string().into(),
                    request_id: "TEST_REQ_ID".to_string(),
                    requester_agent_id: A_1_ID.to_string().into(),
                    query: b"bob".to_vec().into(),
                }),
                Box::new(|_, r| {
                    print_result(r);
                    Ok(())
                }),
            )
            .unwrap();
        self.process();
    }

    pub fn publish_1(&mut self) {
        self.engine1
            .publish(
                Span::fixme(),
                ClientToLib3h::PublishEntry(ProvidedEntryData {
                    space_address: SPACE_ID.to_string().into(),
                    provider_agent_id: A_1_ID.to_string().into(),
                    entry: EntryData {
                        entry_address: ENTRY_ADDR.to_string().into(),
                        aspect_list: vec![EntryAspectData {
                            aspect_address: ASPECT_ADDR.to_string().into(),
                            type_hint: "test".to_string(),
                            aspect: b"bob".to_vec().into(),
                            publish_ts: 0,
                        }],
                    },
                }),
            )
            .unwrap();
        self.process();
        // publish needs an extra 10 process calls
        self.process();
    }
}

fn init_setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    env_logger::builder()
        .default_format_timestamp(false)
        .is_test(false)
        .try_init()
        .unwrap()
}

fn usage() {
    println!(
        r#"USAGE:
  cargo run --bin demo -- --send          # demo direct send
  cargo run --bin demo -- --send --ws     # demo direct send (websocket)
  cargo run --bin demo -- --query         # demo query
  cargo run --bin demo -- --query --ws    # demo query (websocket)
  cargo run --bin demo -- --publish       # demo publish
  cargo run --bin demo -- --publish --ws  # demo publish (websocket)
"#
    );
    std::process::exit(1);
}

fn main() {
    init_setup();

    #[derive(Debug)]
    enum Demo {
        DemoNone,
        DemoSend,
        DemoQuery,
        DemoPublish,
    }
    use Demo::*;

    let mut ws = false;
    let mut demo = DemoNone;

    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--ws" => ws = true,
            "--send" => demo = DemoSend,
            "--query" => demo = DemoQuery,
            "--publish" => demo = DemoPublish,
            _ => {
                println!("unexpected {:?}", a);
                usage();
            }
        }
    }

    if let DemoNone = demo {
        println!("please specify a demo type (--send, --query, etc)");
        usage();
    }

    let mut engines = EngineContainer::new(ws);

    match demo {
        DemoNone => usage(),
        DemoSend => engines.send_1_to_2(),
        DemoQuery => engines.query_1(),
        DemoPublish => engines.publish_1(),
    }
}
