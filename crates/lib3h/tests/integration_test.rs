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
#[macro_use]
extern crate failure;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate multihash;

mod node_mock;
mod test_suites;

use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{RealEngine, RealEngineConfig},
    transport::memory_mock::transport_memory::TransportMemory,
    transport_wss::TlsConfig,
};
use lib3h_crypto_api::{FakeCryptoSystem, InsecureBuffer};
use lib3h_protocol::{network_engine::NetworkEngine, Address, Lib3hResult};
use node_mock::NodeMock;
use test_suites::{two_basic::*, two_get_lists::*, two_spaces::*};
use url::Url;
use utils::constants::*;

//--------------------------------------------------------------------------------------------------
// Logging
//--------------------------------------------------------------------------------------------------

// for this to actually show log entries you also have to run the tests like this:
// RUST_LOG=lib3h=debug cargo test -- --nocapture
fn enable_logging_for_test(enable: bool) {
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
    let engine: RealEngine<TransportMemory, MirrorDht, InsecureBuffer, FakeCryptoSystem> =
        RealEngine::new_mock(config.clone(), name.into(), MirrorDht::new_with_config).unwrap();
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
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}//{}", fn_name, name).as_str()).unwrap(),
        dht_custom_config: vec![],
    };
    NodeMock::new_with_config(name, agent_id_arg, config, construct_mock_engine)
}

#[allow(dead_code)]
fn setup_wss_node(name: &str, agent_id_arg: Address) -> NodeMock {
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap(),
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
    // Terminate nodes
    alex.stop();
    billy.stop();

    Ok(())
}
