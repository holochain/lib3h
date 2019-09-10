use lib3h_ghost_actor::GhostParentWrapper;
use lib3h_protocol::protocol::*;

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWrapper<Core, Context, Engine, EngineError> = GhostParentWrapper<
    Core,
    Context,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;
