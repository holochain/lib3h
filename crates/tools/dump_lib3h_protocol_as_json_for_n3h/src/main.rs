extern crate lib3h_protocol;
extern crate serde_json;

use lib3h_protocol::{
    data_types::*, protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol,
};

pub fn main() {
    // -- client -- //
    println!(" -- client -- ");

    let success_result = Lib3hClientProtocol::SuccessResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec(),
    });

    println!("{}", serde_json::to_string_pretty(&success_result).unwrap());

    let failure_result = Lib3hClientProtocol::FailureResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec(),
    });

    println!("{}", serde_json::to_string_pretty(&failure_result).unwrap());

    let connect = Lib3hClientProtocol::Connect(ConnectData {
        request_id: "rid".to_string(),
        peer_uri: url::Url::parse("hc:id").unwrap(),
        network_id: "nid".to_string(),
    });

    println!("{}", serde_json::to_string_pretty(&connect).unwrap());

    let join_space = Lib3hClientProtocol::JoinSpace(SpaceData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        agent_id: "aid".to_string().into(),
    });

    println!("{}", serde_json::to_string_pretty(&join_space).unwrap());

    let leave_space = Lib3hClientProtocol::LeaveSpace(SpaceData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        agent_id: "aid".to_string().into(),
    });

    println!("{}", serde_json::to_string_pretty(&leave_space).unwrap());

    let send_direct_message = Lib3hClientProtocol::SendDirectMessage(DirectMessageData {
        space_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        to_agent_id: "aid".to_string().into(),
        from_agent_id: "aid".to_string().into(),
        content: b"yo".to_vec(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&send_direct_message).unwrap()
    );

    let handle_send_direct_message_result =
        Lib3hClientProtocol::HandleSendDirectMessageResult(DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_send_direct_message_result).unwrap()
    );

    let fetch_entry = Lib3hClientProtocol::FetchEntry(FetchEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        provider_agent_id: "aid".to_string().into(),
        aspect_address_list: Some(vec!["adr".to_string().into()]),
    });

    println!("{}", serde_json::to_string_pretty(&fetch_entry).unwrap());

    let handle_fetch_entry_result =
        Lib3hClientProtocol::HandleFetchEntryResult(FetchEntryResultData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            entry: EntryData {
                entry_address: "adr".to_string().into(),
                aspect_list: vec![EntryAspectData {
                    aspect_address: "adr".to_string().into(),
                    type_hint: "hint".to_string(),
                    aspect: b"yo".to_vec(),
                    publish_ts: 42,
                }],
            },
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_fetch_entry_result).unwrap()
    );

    let publish_entry = Lib3hClientProtocol::PublishEntry(ProvidedEntryData {
        space_address: "adr".to_string().into(),
        provider_agent_id: "aid".to_string().into(),
        entry: EntryData {
            entry_address: "adr".to_string().into(),
            aspect_list: vec![EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec(),
                publish_ts: 42,
            }],
        },
    });

    println!("{}", serde_json::to_string_pretty(&publish_entry).unwrap());

    let hold_entry = Lib3hClientProtocol::HoldEntry(ProvidedEntryData {
        space_address: "adr".to_string().into(),
        provider_agent_id: "aid".to_string().into(),
        entry: EntryData {
            entry_address: "adr".to_string().into(),
            aspect_list: vec![EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec(),
                publish_ts: 42,
            }],
        },
    });

    println!("{}", serde_json::to_string_pretty(&hold_entry).unwrap());

    let query_entry = Lib3hClientProtocol::QueryEntry(QueryEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        requester_agent_id: "aid".to_string().into(),
        query: b"yo".to_vec(),
    });

    println!("{}", serde_json::to_string_pretty(&query_entry).unwrap());

    let handle_query_entry_result =
        Lib3hClientProtocol::HandleQueryEntryResult(QueryEntryResultData {
            space_address: "adr".to_string().into(),
            entry_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            requester_agent_id: "aid".to_string().into(),
            responder_agent_id: "aid".to_string().into(),
            query_result: b"yo".to_vec(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_query_entry_result).unwrap()
    );

    let handle_get_authoring_entry_list_result =
        Lib3hClientProtocol::HandleGetAuthoringEntryListResult(EntryListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            address_map: [("adr".to_string().into(), vec!["adr".to_string().into()])]
                .iter()
                .cloned()
                .collect(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_get_authoring_entry_list_result).unwrap()
    );

    let handle_get_gossiping_entry_list_result =
        Lib3hClientProtocol::HandleGetGossipingEntryListResult(EntryListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            address_map: [("adr".to_string().into(), vec!["adr".to_string().into()])]
                .iter()
                .cloned()
                .collect(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_get_gossiping_entry_list_result).unwrap()
    );

    let shutdown = Lib3hClientProtocol::Shutdown;

    println!("{}", serde_json::to_string_pretty(&shutdown).unwrap());

    // -- server -- //

    println!(" -- server -- ");

    let success_result = Lib3hServerProtocol::SuccessResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec(),
    });

    println!("{}", serde_json::to_string_pretty(&success_result).unwrap());

    let failure_result = Lib3hServerProtocol::FailureResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec(),
    });

    println!("{}", serde_json::to_string_pretty(&failure_result).unwrap());

    let connected = Lib3hServerProtocol::Connected(ConnectedData {
        request_id: "rid".to_string(),
        uri: url::Url::parse("hc:id").unwrap(),
    });

    println!("{}", serde_json::to_string_pretty(&connected).unwrap());

    let disconnected = Lib3hServerProtocol::Disconnected(DisconnectedData {
        network_id: "nid".to_string(),
    });

    println!("{}", serde_json::to_string_pretty(&disconnected).unwrap());

    let send_direct_message_result =
        Lib3hServerProtocol::SendDirectMessageResult(DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&send_direct_message_result).unwrap()
    );

    let handle_send_direct_message =
        Lib3hServerProtocol::HandleSendDirectMessage(DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_send_direct_message).unwrap()
    );

    let fetch_entry_result = Lib3hServerProtocol::FetchEntryResult(FetchEntryResultData {
        space_address: "adr".to_string().into(),
        provider_agent_id: "aid".to_string().into(),
        request_id: "rid".to_string(),
        entry: EntryData {
            entry_address: "adr".to_string().into(),
            aspect_list: vec![EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec(),
                publish_ts: 42,
            }],
        },
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&fetch_entry_result).unwrap()
    );

    let handle_fetch_entry = Lib3hServerProtocol::HandleFetchEntry(FetchEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        provider_agent_id: "aid".to_string().into(),
        aspect_address_list: Some(vec!["adr".to_string().into()]),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_fetch_entry).unwrap()
    );

    let handle_store_entry_aspect =
        Lib3hServerProtocol::HandleStoreEntryAspect(StoreEntryAspectData {
            request_id: "rid".to_string(),
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            entry_address: "adr".to_string().into(),
            entry_aspect: EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec(),
                publish_ts: 42,
            },
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_store_entry_aspect).unwrap()
    );

    let handle_drop_entry = Lib3hServerProtocol::HandleDropEntry(DropEntryData {
        space_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        entry_address: "adr".to_string().into(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_drop_entry).unwrap()
    );

    let handle_query_entry = Lib3hServerProtocol::HandleQueryEntry(QueryEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        requester_agent_id: "aid".to_string().into(),
        query: b"yo".to_vec(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_query_entry).unwrap()
    );

    let query_entry_result = Lib3hServerProtocol::QueryEntryResult(QueryEntryResultData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        requester_agent_id: "aid".to_string().into(),
        responder_agent_id: "aid".to_string().into(),
        query_result: b"yo".to_vec(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&query_entry_result).unwrap()
    );

    let handle_get_authoring_entry_list =
        Lib3hServerProtocol::HandleGetAuthoringEntryList(GetListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_get_authoring_entry_list).unwrap()
    );

    let handle_get_gossiping_entry_list =
        Lib3hServerProtocol::HandleGetGossipingEntryList(GetListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&handle_get_gossiping_entry_list).unwrap()
    );

    let terminated = Lib3hServerProtocol::Terminated;

    println!("{}", serde_json::to_string_pretty(&terminated).unwrap());

    let p2p_ready = Lib3hServerProtocol::P2pReady;

    println!("{}", serde_json::to_string_pretty(&p2p_ready).unwrap());
}
