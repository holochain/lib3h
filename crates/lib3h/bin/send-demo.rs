#[macro_use]
extern crate detach;

use detach::Detach;
use holochain_tracing::*;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{engine_actor::*, *},
    error::*,
};
use lib3h_protocol::{data_types::*, protocol::*};
use lib3h_sodium::SodiumCryptoSystem;
use lib3h_zombie_actor::*;
use url::Url;

static NET_ID: &'static str = "send-demo-network";
static SPACE_ID: &'static str = "send-demo-space";
static A_1_ID: &'static str = "agent-1-id";
static A_2_ID: &'static str = "agent-2-id";

#[allow(dead_code)]
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
    engine1_addr: Url,
    engine2: Detach<GhostEngineParentWrapper<EngineContainer<E>, E, Lib3hError>>,
    engine2_addr: Url,
}

impl<'lt> EngineContainer<GhostEngine<'lt>> {
    pub fn new() -> Self {
        let crypto = Box::new(SodiumCryptoSystem::new());

        let config = EngineConfig {
            network_id: GatewayId {
                nickname: NET_ID.to_string(),
                id: NET_ID.to_string().into(),
            },
            transport_configs: vec![TransportConfig::Memory("send-demo".to_string())],
            bootstrap_nodes: vec![],
            work_dir: std::path::PathBuf::new(),
            log_level: 'd',
            bind_url: Url::parse("none:").unwrap(),
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
        out.process();
        out.process();
        out
    }

    pub fn process(&mut self) {
        detach_run!(self.engine1, |e| { e.process(self) }).unwrap();
        detach_run!(self.engine2, |e| { e.process(self) }).unwrap();
        for mut msg in self.engine1.drain_messages() {
            let payload = msg.take_message();
            println!("1 got: {:?}", payload);
        }
        for mut msg in self.engine2.drain_messages() {
            let payload = msg.take_message();
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
                _ => println!("2 got: {:?}", payload),
            }
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
                    println!("WE GOT A RESULT!!!: {:?}", r);
                    Ok(())
                }),
            )
            .unwrap();
        self.process();
        self.process();
        self.process();
        self.process();
        self.process();
    }
}

pub fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    let _ = env_logger::builder()
        .default_format_timestamp(false)
        .is_test(false)
        .try_init();

    let mut engines = EngineContainer::new();
    engines.send_1_to_2();
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    // node 2 should have message now-ish... run a couple more to get it back
    engines.process();
    engines.process();
    engines.process();
    engines.process();
    // now back to node 1?
    engines.process();
    engines.process();
    engines.process();
    engines.process();
}
