extern crate rand;
extern crate xoroshiro128;

pub mod constants;
#[macro_use]
pub mod predicate;

pub mod seeded_prng;

#[macro_use]
pub mod processor_harness;

#[allow(unused_macros)]
macro_rules! assert_process_success {
    ($node: ident, $req: ident) => {
        let (did_work, srv_msg_list) = $node.process().unwrap();
        assert!(did_work);
// TODO - fixed with new test macros
        // assert_eq!(srv_msg_list.len(), 1);
        // let msg_1 = &srv_msg_list[0];
        // match msg_1 {
        //     Lib3hServerProtocol::SuccessResult(response) => assert_eq!(response.request_id, $req),
        //     Lib3hServerProtocol::FailureResult(response) => {
        //         let content = std::str::from_utf8(response.result_info.as_slice()).unwrap();
        //         panic!("Received FailureResult: {}", content);
        //     }
        //     _ => panic!("Received unexpected Protocol message type"),
        // }
    };
}
