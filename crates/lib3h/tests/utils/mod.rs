pub mod constants;
#[macro_use]
pub mod predicate;

#[allow(unused_macros)]
macro_rules! assert_process_success {
    ($node: ident, $req: ident) => {
        let (did_work, srv_msg_list) = $node.process().unwrap();
        assert!(did_work);
        assert_eq!(srv_msg_list.len(), 1);
        let msg_1 = &srv_msg_list[0];
        if let Lib3hServerProtocol::SuccessResult(response) = msg_1 {
            assert_eq!(response.request_id, $req);
        } else {
            if let Lib3hServerProtocol::FailureResult(response) = msg_1 {
                let content = std::str::from_utf8(response.result_info.as_slice()).unwrap();
                panic!("Received FailureResult: {}", content);
            } else {
                panic!("Received unexpected Protocol message type");
            }
        }
    };
}
