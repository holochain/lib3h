#[macro_use]
extern crate hexf;

#[macro_use]
mod utils;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate unwrap_to;
extern crate backtrace;
#[macro_use]
extern crate log;
extern crate holochain_persistence_api;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate multihash;

mod node_mock;
mod test_suites;

use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{RealEngine, RealEngineConfig},
    error::Lib3hResult,
    transport_wss::TlsConfig,
};
use lib3h_protocol::{network_engine::NetworkEngine, Address};
use node_mock::NodeMock;
use std::path::PathBuf;
use test_suites::{
    three_basic::*, two_basic::*, two_connection::*, two_get_lists::*, two_spaces::*,
};
use url::Url;
use utils::constants::*;

//--------------------------------------------------------------------------------------------------
// Logging
//--------------------------------------------------------------------------------------------------

// for this to actually show log entries you also have to run the tests like this:
// RUST_LOG=lib3h=debug cargo test -- --nocapture
fn enable_logging_for_test(enable: bool) {
    // wait a bit because of non monotonic clock,
    // otherwise we could get negative substraction panics
    // TODO #211
    std::thread::sleep(std::time::Duration::from_millis(5));
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "trace");
    }
    let _ = env_logger::builder()
        .default_format_timestamp(false)
        .default_format_module_path(false)
        .is_test(enable)
        .try_init();
}

//--------------------------------------------------------------------------------------------------
// Engine factories
//--------------------------------------------------------------------------------------------------

fn construct_mock_engine(
    config: &RealEngineConfig,
    name: &str,
) -> Lib3hResult<Box<dyn NetworkEngine>> {
    let engine: RealEngine<MirrorDht> = RealEngine::new_mock(
        Box::new(lib3h_sodium::SodiumCryptoSystem::new()),
        config.clone(),
        name.into(),
        MirrorDht::new_with_config,
    )
    .unwrap();
    let p2p_binding = engine.advertise();
    println!(
        "construct_mock_engine(): test engine for {}, advertise: {}",
        name, p2p_binding
    );
    Ok(Box::new(engine))
}

//--------------------------------------------------------------------------------------------------
// Node Setup
//--------------------------------------------------------------------------------------------------

pub type NodeFactory = fn(name: &str, agent_id_arg: Address) -> NodeMock;

fn setup_memory_node(name: &str, agent_id_arg: Address, fn_name: &str) -> NodeMock {
    let fn_name = fn_name.replace("::", "__");
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "mem".into(),
        bootstrap_nodes: vec![],
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}/{}", fn_name, name).as_str()).unwrap(),
        dht_gossip_interval: 500,
        dht_timeout_threshold: 3000,
        dht_custom_config: vec![],
    };
    NodeMock::new_with_config(name, agent_id_arg, config, construct_mock_engine)
}

fn setup_wss_node(
    name: &str,
    agent_id_arg: Address,
    tls_config: TlsConfig,
    fn_name: &str,
) -> NodeMock {
    let fn_name = fn_name.replace("::", "__");
    let port = generate_port();
    let protocol = match tls_config {
        TlsConfig::Unencrypted => "ws",
        TlsConfig::SuppliedCertificate(_) | TlsConfig::FakeServer => "wss",
    };
    let bind_url = Url::parse(format!("{}://127.0.0.1:{}/{}", protocol, port, fn_name).as_str())
        .expect("invalid web socket url");

    let config = RealEngineConfig {
        tls_config: tls_config,
        socket_type: protocol.into(),
        bootstrap_nodes: vec![],
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url,
        dht_gossip_interval: 500,
        dht_timeout_threshold: 3005,
        dht_custom_config: vec![],
    };
    NodeMock::new_with_config(name, agent_id_arg, config, construct_mock_engine)
}

//--------------------------------------------------------------------------------------------------
// Utils
//--------------------------------------------------------------------------------------------------

/// Get function name as a String by using backtrace
fn fn_name(test_fn: *mut std::os::raw::c_void) -> String {
    let mut fn_name = String::new();
    backtrace::resolve(test_fn, |symbol| {
        let mut full_name = symbol.name().unwrap().as_str().unwrap().to_string();
        fn_name = full_name.split_off("integration_test::test_suites::".to_string().len());
    });
    fn_name
}

/// Print name of test function
fn print_test_name(print_str: &str, test_fn: *mut std::os::raw::c_void) {
    let mut fn_name = fn_name(test_fn);
    fn_name.push_str("()");
    println!("{}{}", print_str, fn_name);
}

//--------------------------------------------------------------------------------------------------
// Test launchers
//--------------------------------------------------------------------------------------------------

// -- Memory Transport Tests --
#[test]
fn test_two_memory_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_BASIC_TEST_FNS.iter() {
        launch_two_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}

#[test]
fn test_two_memory_nodes_get_lists_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_GET_LISTS_TEST_FNS.iter() {
        launch_two_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}

#[test]
fn test_two_memory_nodes_spaces_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_SPACES_TEST_FNS.iter() {
        launch_two_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}

#[test]
fn test_three_memory_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in THREE_NODES_BASIC_TEST_FNS.iter() {
        launch_three_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}
#[test]
fn test_two_memory_nodes_connection_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_CONNECTION_TEST_FNS.iter() {
        launch_two_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}

// Do general test with config
fn launch_two_memory_nodes_test(test_fn: TwoNodesTestFn, can_setup: bool) -> Result<(), ()> {
    let test_fn_ptr = test_fn as *mut std::os::raw::c_void;
    println!("");
    print_test_name("IN-MEMORY TWO NODES TEST: ", test_fn_ptr);
    println!("========================");

    // Setup
    let mut alex = setup_memory_node("alex", ALEX_AGENT_ID.clone(), &fn_name(test_fn_ptr));
    let mut billy = setup_memory_node("billy", BILLY_AGENT_ID.clone(), &fn_name(test_fn_ptr));
    if can_setup {
        setup_two_nodes(&mut alex, &mut billy);
    }

    // Execute test
    test_fn(&mut alex, &mut billy);

    // Wrap-up test
    println!("========================");
    print_test_name("IN-MEMORY TWO NODES TEST END: ", test_fn_ptr);

    // Done
    Ok(())
}

// Do general test with config
fn launch_three_memory_nodes_test(test_fn: ThreeNodesTestFn, can_setup: bool) -> Result<(), ()> {
    let test_fn_ptr = test_fn as *mut std::os::raw::c_void;
    println!("");
    print_test_name("IN-MEMORY THREE NODES TEST: ", test_fn_ptr);
    println!("==========================");

    // Setup
    let mut alex = setup_memory_node("alex", ALEX_AGENT_ID.clone(), &fn_name(test_fn_ptr));
    let mut billy = setup_memory_node("billy", BILLY_AGENT_ID.clone(), &fn_name(test_fn_ptr));
    let mut camille = setup_memory_node("camille", CAMILLE_AGENT_ID.clone(), &fn_name(test_fn_ptr));
    if can_setup {
        setup_three_nodes(&mut alex, &mut billy, &mut camille);
    }

    // Execute test
    test_fn(&mut alex, &mut billy, &mut camille);

    // Wrap-up test
    println!("==========================");
    print_test_name("IN-MEMORY THREE NODES TEST END: ", test_fn_ptr);

    // Done
    Ok(())
}

// -- Wss Transport Tests --
#[test]
fn test_two_wss_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_BASIC_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::Unencrypted, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_nodes_get_lists_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_GET_LISTS_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::Unencrypted, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_nodes_spaces_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_SPACES_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::Unencrypted, *can_setup).unwrap();
    }
}

#[test]
fn test_three_wss_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in THREE_NODES_BASIC_TEST_FNS.iter() {
        launch_three_wss_nodes_test(*test_fn, TlsConfig::Unencrypted, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_nodes_connection_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_CONNECTION_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::Unencrypted, *can_setup).unwrap();
    }
}

// -- Wss+Tls Transport Tests --
#[test]
fn test_two_wss_tls_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_BASIC_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::FakeServer, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_tls_nodes_get_lists_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_GET_LISTS_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::FakeServer, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_tls_nodes_spaces_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_SPACES_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::FakeServer, *can_setup).unwrap();
    }
}

#[test]
fn test_three_wss_tls_nodes_basic_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in THREE_NODES_BASIC_TEST_FNS.iter() {
        launch_three_wss_nodes_test(*test_fn, TlsConfig::FakeServer, *can_setup).unwrap();
    }
}

#[test]
fn test_two_wss_tls_nodes_connection_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_CONNECTION_TEST_FNS.iter() {
        launch_two_wss_nodes_test(*test_fn, TlsConfig::FakeServer, *can_setup).unwrap();
    }
}

// Do general test with config
fn launch_two_wss_nodes_test(
    test_fn: TwoNodesTestFn,
    tls_config: TlsConfig,
    can_setup: bool,
) -> Result<(), ()> {
    let test_fn_ptr = test_fn as *mut std::os::raw::c_void;
    println!("");
    print_test_name(
        format!("WSS TWO NODES TEST ({:?}): ", tls_config.clone()).as_str(),
        test_fn_ptr,
    );
    println!("========================");

    // Setup
    let mut alex = setup_wss_node(
        "alex",
        ALEX_AGENT_ID.clone(),
        tls_config.clone(),
        &fn_name(test_fn_ptr),
    );
    let mut billy = setup_wss_node(
        "billy",
        BILLY_AGENT_ID.clone(),
        tls_config.clone(),
        &fn_name(test_fn_ptr),
    );
    if can_setup {
        setup_two_nodes(&mut alex, &mut billy);
    }

    // Execute test
    test_fn(&mut alex, &mut billy);

    // Wrap-up test
    println!("========================");
    print_test_name(
        format!("WSS TWO NODES TEST END ({:?}):", tls_config.clone()).as_str(),
        test_fn_ptr,
    );

    // Done
    Ok(())
}

// Do general test with config
fn launch_three_wss_nodes_test(
    test_fn: ThreeNodesTestFn,
    tls_config: TlsConfig,
    can_setup: bool,
) -> Result<(), ()> {
    let test_fn_ptr = test_fn as *mut std::os::raw::c_void;
    println!("");
    print_test_name(
        format!("WSS THREE NODES TEST ({:?}):", tls_config.clone()).as_str(),
        test_fn_ptr,
    );

    println!("==========================");

    // Setup
    let mut alex = setup_wss_node(
        "alex",
        ALEX_AGENT_ID.clone(),
        tls_config.clone(),
        &fn_name(test_fn_ptr),
    );
    let mut billy = setup_wss_node(
        "billy",
        BILLY_AGENT_ID.clone(),
        tls_config.clone(),
        &fn_name(test_fn_ptr),
    );
    let mut camille = setup_wss_node(
        "camille",
        CAMILLE_AGENT_ID.clone(),
        tls_config.clone(),
        &fn_name(test_fn_ptr),
    );
    if can_setup {
        setup_three_nodes(&mut alex, &mut billy, &mut camille);
    }

    // Execute test
    test_fn(&mut alex, &mut billy, &mut camille);

    // Wrap-up test
    println!("==========================");
    print_test_name(
        format!("WSS THREE NODES TEST END ({:?}):", tls_config.clone()).as_str(),
        test_fn_ptr,
    );

    // Done
    Ok(())
}
