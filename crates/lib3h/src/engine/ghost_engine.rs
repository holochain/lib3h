use lib3h_ghost_actor::GhostParentWrapper;
use lib3h_protocol::protocol::*;
/// the context when making a request from core
/// this is always the request_id
pub struct CoreRequestContext(String);
impl CoreRequestContext {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
    pub fn get_request_id(&self) -> String {
        self.0.clone()
    }
}

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWapper<Core, Engine, EngineError> = GhostParentWrapper<
    Core,
    CoreRequestContext,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;
