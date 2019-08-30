//! Let's say we have two agentIds: a1 and a2
//! a1 is running on machineId: m1
//! a2 is running on machineId: m2
//!
//! The AgentSpaceGateway will wrap messages in a p2p_proto direct message:
//!   DirectMessage {
//!     space_address: "Qmyada",
//!     to_agent_id: "a2",
//!     from_agent_id: "a1",
//!     payload: <...>,
//!   }
//!
//! Then send it to the machine id:
//!   dest: "m2", payload: <above, but binary>
//!
//! When the multiplexer receives data (at the network/machine gateway),
//! if it is any other p2p_proto message, it will be forwarded to
//! the engine or network gateway. If it is a direct message, it will be
//! send to the appropriate Route / AgentSpaceGateway

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocalRouteSpec {
    pub space_address: String,
    pub local_agent_id: String,
}

mod route;
pub use route::TransportMultiplexRoute;

mod mplex;
pub use mplex::TransportMultiplex;
