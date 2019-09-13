
use crate::lib3h_simchat::Lib3hSimChatState;
use super::current_timestamp;
use crate::simchat::{ChatEvent, SimChatMessage};
use lib3h_protocol::{
    data_types::*,
    protocol::{ClientToLib3h, Lib3hToClient, Lib3hToClientResponse},
};
use lib3h_ghost_actor::GhostMessage;
use lib3h::error::Lib3hError;

pub fn handle_and_convert_lib3h_event(
	mut engine_message: GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>,
	state: &mut Lib3hSimChatState,
) -> Option<ChatEvent> {
match engine_message.take_message() {
	    Some(Lib3hToClient::HandleQueryEntry(QueryEntryData {
	        space_address,
	        entry_address,
	        request_id,
	        requester_agent_id,
	        query: _,
	    })) => {
	        // respond with your ID in this space
	        let responder_agent_id = state.spaces.get(&space_address).unwrap().agent_id.clone();
	        if let Some(entry) = state.cas.get(&entry_address) {
	            // load and return data from the CAS
	            engine_message.respond(Ok(Lib3hToClientResponse::HandleQueryEntryResult(QueryEntryResultData {
	                request_id,
	                entry_address,
	                requester_agent_id: requester_agent_id.clone(),
	                space_address: space_address.clone(),
	                responder_agent_id,
	                query_result: entry.into(),
	            }))).ok();
	        } else {
	            // is this the correct way to respond to a missing entry?
	            engine_message.respond(Err(Lib3hError::new_other("Entry not found"))).ok();
	        }
	        None
	    },
	    Some(Lib3hToClient::HandleStoreEntryAspect(StoreEntryAspectData {
	        entry_address,
	        entry_aspect,
	        space_address: _,
	        ..
	    })) => {
	        // store some data in the CAS at the given address
	        state.cas.insert(entry_address, String::from_utf8(entry_aspect.aspect.as_bytes()).unwrap());
	        engine_message.respond(Ok(Lib3hToClientResponse::HandleStoreEntryAspectResult)).ok();
	        None
	    },
	    Some(Lib3hToClient::HandleGetAuthoringEntryList(GetListData {
	        request_id,
	        space_address,
	        provider_agent_id,
	    })) => {
	        engine_message.respond(Ok(Lib3hToClientResponse::HandleGetAuthoringEntryListResult(EntryListData {
	            request_id,
	            space_address,
	            provider_agent_id,
	            address_map: state.author_list.clone(),
	        }))).ok();
	        None
	    },
	    Some(Lib3hToClient::HandleGetGossipingEntryList(GetListData {
	        request_id,
	        space_address,
	        provider_agent_id,
	    })) => {
	        engine_message.respond(Ok(Lib3hToClientResponse::HandleGetAuthoringEntryListResult(EntryListData {
	            request_id,
	            space_address,
	            provider_agent_id,
	            address_map: state.gossip_list.clone(),
	        }))).ok(); 
	        None
	    },
	    Some(Lib3hToClient::HandleSendDirectMessage(message_data)) => {
	        Some(ChatEvent::ReceiveDirectMessage(SimChatMessage {
	            from_agent: message_data.from_agent_id.to_string(),
	            payload: "message from engine".to_string(),
	            timestamp: current_timestamp(),
	        }))
	    }
	    Some(Lib3hToClient::Disconnected(_)) => Some(ChatEvent::Disconnected),
	    Some(_) => {None}, // event we don't care about
	    None => {None}, // there was nothing in the message
	}
}
