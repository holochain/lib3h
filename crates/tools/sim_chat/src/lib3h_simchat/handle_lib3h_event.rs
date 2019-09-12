
use crate::current_timestamp;
use crate::simchat::{ChatEvent, SimChatMessage};
use lib3h_protocol::{
    data_types::{QueryEntryData, QueryEntryResultData},
    protocol::{ClientToLib3h, Lib3hToClient, Lib3hToClientResponse},
};
use lib3h_ghost_actor::GhostMessage;
use lib3h::error::Lib3hError;

pub fn handle_and_convert_lib3h_event(mut engine_message: GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>) -> Option<ChatEvent> {
	match engine_message.take_message() {
	    Some(Lib3hToClient::HandleQueryEntry(QueryEntryData {
	        space_address,
	        entry_address,
	        request_id,
	        requester_agent_id,
	        query: _,
	    })) => {
	        engine_message.respond(Ok(Lib3hToClientResponse::HandleQueryEntryResult(QueryEntryResultData {
	            request_id,
	            entry_address,
	            requester_agent_id: requester_agent_id.clone(),
	            space_address: space_address.clone(),
	            responder_agent_id: requester_agent_id,
	            query_result: Vec::new().into(),
	        }))).ok();
	        None
	    },
	    Some(Lib3hToClient::HandleStoreEntryAspect{..}) => {
	        None
	    },
	    Some(Lib3hToClient::HandleGetAuthoringEntryList{..}) => {
	        None
	    },
	    Some(Lib3hToClient::HandleGetGossipingEntryList{..}) => {
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
