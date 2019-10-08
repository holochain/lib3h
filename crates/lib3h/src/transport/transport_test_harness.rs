/// Macros to process a transport and await certain conditions or fail
///
#[allow(dead_code)]
pub const DEFAULT_PORT: u16 = 0;
#[allow(dead_code)]
fn port_is_available(port: u16) -> bool {
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[allow(dead_code)]
pub fn get_available_port(start: u16) -> Option<u16> {
    (start..65535).find(|port| port_is_available(*port))
}

/// Waits until a bind result is produced by a ghost transport
#[macro_export]
#[allow(unused_macros)]
macro_rules! wait_for_bind_result {
    {
        $actor: ident,
        $can_track: ident,
        $init_url: expr
    } => {{
        let options = $crate::lib3h_ghost_actor::ghost_test_harness::ProcessingOptions::default();
        $crate::wait_for_bind_result!($actor, $can_track, $init_url, options)
    }};
    {
        $actor: ident,
        $can_track: ident,
        $init_url: expr,
        $options: expr
    } => {{
        let init_transport_address: $crate::lib3h_protocol::uri::Lib3hUri = $init_url.into();

        let request_fn = Box::new(|transport_address: $crate::lib3h_protocol::uri::Lib3hUri| {
            let old_port = transport_address.port()
                .unwrap_or_else(|| $crate::transport::transport_test_harness::DEFAULT_PORT);
            let port = $crate::transport::transport_test_harness::get_available_port(old_port + 1)
                .expect("Must be able to find free port");

            let expected_transport_address = transport_address.with_port(port);

            let request = RequestToChild::Bind {
                spec: expected_transport_address.clone(),
            };
            let re = format!(
                "Response\\(Ok\\(Bind\\(BindResultData \\{{ bound_url: Lib3hUri\\(\"{:?}\"\\) \\}}\\)\\)\\)",
                expected_transport_address.to_string()
            );

            (request.clone(), re, expected_transport_address.clone())
        });

        $crate::lib3h_ghost_actor::wait1_for_repeatable_callback!($actor, $can_track, request_fn,
            init_transport_address, $options)
    }}
}
