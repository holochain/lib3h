extern crate lib3h_protocol;
extern crate serde_json;

use lib3h_protocol::{
    data_types::*, protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol,
};

fn test_client(m: Lib3hClientProtocol) {
    let json = serde_json::to_string_pretty(&m).unwrap();
    println!("{}", &json);
    let _: Lib3hClientProtocol = serde_json::from_str(&json).unwrap();
}

fn test_server(m: Lib3hServerProtocol) {
    let json = serde_json::to_string_pretty(&m).unwrap();
    println!("{}", &json);
    let _: Lib3hServerProtocol = serde_json::from_str(&json).unwrap();
}

pub fn dump_lib3h_protocol_as_json_for_n3h() {
    // -- client -- //
    println!("\n-- client --\n");

    test_client(Lib3hClientProtocol::SuccessResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec().into(),
    }));

    test_client(Lib3hClientProtocol::FailureResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec().into(),
    }));

    test_client(Lib3hClientProtocol::Connect(ConnectData {
        request_id: "rid".to_string(),
        peer_uri: url::Url::parse("hc:id").expect("hc:id to be a valid url"),
        network_id: "nid".to_string(),
    }));

    test_client(Lib3hClientProtocol::JoinSpace(SpaceData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        agent_id: "aid".to_string().into(),
    }));

    test_client(Lib3hClientProtocol::LeaveSpace(SpaceData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        agent_id: "aid".to_string().into(),
    }));

    test_client(Lib3hClientProtocol::SendDirectMessage(DirectMessageData {
        space_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        to_agent_id: "aid".to_string().into(),
        from_agent_id: "aid".to_string().into(),
        content: b"yo".to_vec().into(),
    }));

    test_client(Lib3hClientProtocol::HandleSendDirectMessageResult(
        DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec().into(),
        },
    ));

    test_client(Lib3hClientProtocol::FetchEntry(FetchEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        provider_agent_id: "aid".to_string().into(),
        aspect_address_list: Some(vec!["adr".to_string().into()]),
    }));

    test_client(Lib3hClientProtocol::HandleFetchEntryResult(
        FetchEntryResultData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            entry: EntryData {
                entry_address: "adr".to_string().into(),
                aspect_list: vec![EntryAspectData {
                    aspect_address: "adr".to_string().into(),
                    type_hint: "hint".to_string(),
                    aspect: b"yo".to_vec().into(),
                    publish_ts: 42,
                }],
            },
        },
    ));

    test_client(Lib3hClientProtocol::PublishEntry(ProvidedEntryData {
        space_address: "adr".to_string().into(),
        provider_agent_id: "aid".to_string().into(),
        entry: EntryData {
            entry_address: "adr".to_string().into(),
            aspect_list: vec![EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec().into(),
                publish_ts: 42,
            }],
        },
    }));

    test_client(Lib3hClientProtocol::HoldEntry(ProvidedEntryData {
        space_address: "adr".to_string().into(),
        provider_agent_id: "aid".to_string().into(),
        entry: EntryData {
            entry_address: "adr".to_string().into(),
            aspect_list: vec![EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec().into(),
                publish_ts: 42,
            }],
        },
    }));

    test_client(Lib3hClientProtocol::QueryEntry(QueryEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        requester_agent_id: "aid".to_string().into(),
        query: b"yo".to_vec().into(),
    }));

    test_client(Lib3hClientProtocol::HandleQueryEntryResult(
        QueryEntryResultData {
            space_address: "adr".to_string().into(),
            entry_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            requester_agent_id: "aid".to_string().into(),
            responder_agent_id: "aid".to_string().into(),
            query_result: b"yo".to_vec().into(),
        },
    ));

    test_client(Lib3hClientProtocol::HandleGetAuthoringEntryListResult(
        EntryListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            address_map: [("adr".to_string().into(), vec!["adr".to_string().into()])]
                .iter()
                .cloned()
                .collect(),
        },
    ));

    test_client(Lib3hClientProtocol::HandleGetGossipingEntryListResult(
        EntryListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            address_map: [("adr".to_string().into(), vec!["adr".to_string().into()])]
                .iter()
                .cloned()
                .collect(),
        },
    ));

    test_client(Lib3hClientProtocol::Shutdown);

    // -- server -- //

    println!("\n-- server --\n");

    test_server(Lib3hServerProtocol::SuccessResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec().into(),
    }));

    test_server(Lib3hServerProtocol::FailureResult(GenericResultData {
        request_id: "rid".to_string(),
        space_address: "adr".to_string().into(),
        to_agent_id: "aid".to_string().into(),
        result_info: b"yo".to_vec().into(),
    }));

    test_server(Lib3hServerProtocol::Connected(ConnectedData {
        request_id: "rid".to_string(),
        uri: url::Url::parse("hc:id").expect("hc:id is a valid url"),
    }));

    test_server(Lib3hServerProtocol::Disconnected(DisconnectedData {
        network_id: "nid".to_string(),
    }));

    test_server(Lib3hServerProtocol::SendDirectMessageResult(
        DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec().into(),
        },
    ));

    test_server(Lib3hServerProtocol::HandleSendDirectMessage(
        DirectMessageData {
            space_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            to_agent_id: "aid".to_string().into(),
            from_agent_id: "aid".to_string().into(),
            content: b"yo".to_vec().into(),
        },
    ));

    test_server(Lib3hServerProtocol::FetchEntryResult(
        FetchEntryResultData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
            entry: EntryData {
                entry_address: "adr".to_string().into(),
                aspect_list: vec![EntryAspectData {
                    aspect_address: "adr".to_string().into(),
                    type_hint: "hint".to_string(),
                    aspect: b"yo".to_vec().into(),
                    publish_ts: 42,
                }],
            },
        },
    ));

    test_server(Lib3hServerProtocol::HandleFetchEntry(FetchEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        provider_agent_id: "aid".to_string().into(),
        aspect_address_list: Some(vec!["adr".to_string().into()]),
    }));

    test_server(Lib3hServerProtocol::HandleStoreEntryAspect(
        StoreEntryAspectData {
            request_id: "rid".to_string(),
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            entry_address: "adr".to_string().into(),
            entry_aspect: EntryAspectData {
                aspect_address: "adr".to_string().into(),
                type_hint: "hint".to_string(),
                aspect: b"yo".to_vec().into(),
                publish_ts: 42,
            },
        },
    ));

    test_server(Lib3hServerProtocol::HandleDropEntry(DropEntryData {
        space_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        entry_address: "adr".to_string().into(),
    }));

    test_server(Lib3hServerProtocol::HandleQueryEntry(QueryEntryData {
        space_address: "adr".to_string().into(),
        entry_address: "adr".to_string().into(),
        request_id: "rid".to_string(),
        requester_agent_id: "aid".to_string().into(),
        query: b"yo".to_vec().into(),
    }));

    test_server(Lib3hServerProtocol::QueryEntryResult(
        QueryEntryResultData {
            space_address: "adr".to_string().into(),
            entry_address: "adr".to_string().into(),
            request_id: "rid".to_string(),
            requester_agent_id: "aid".to_string().into(),
            responder_agent_id: "aid".to_string().into(),
            query_result: b"yo".to_vec().into(),
        },
    ));

    test_server(Lib3hServerProtocol::HandleGetAuthoringEntryList(
        GetListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
        },
    ));

    test_server(Lib3hServerProtocol::HandleGetGossipingEntryList(
        GetListData {
            space_address: "adr".to_string().into(),
            provider_agent_id: "aid".to_string().into(),
            request_id: "rid".to_string(),
        },
    ));

    test_server(Lib3hServerProtocol::Terminated);

    test_server(Lib3hServerProtocol::P2pReady);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_generate_and_parse_json() {
        dump_lib3h_protocol_as_json_for_n3h();
    }
}
