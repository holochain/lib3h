// ignore all linting on auto-generated code
#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
/// our ghost-actor code-generated keystore protocol
pub mod keystore_protocol {
    include!(concat!(env!("OUT_DIR"), "/keystore_protocol.rs"));
}

use ghost_actor::prelude::*;
use keystore_protocol::*;

/// a stub actor implementation of the ghost actor 2.0 KeystoreProtocol
pub struct Keystore2ActorStub<'lt, S: GhostSystemRef<'lt>> {
    _sys_ref: GhostActorSystem<'lt, Self, S>,
    _owner_ref:
        GhostEndpointFull<'lt, KeystoreProtocol, (), Self, KeystoreActorHandler<'lt, Self>, S>,
}

impl<'lt, S: GhostSystemRef<'lt>> Keystore2ActorStub<'lt, S> {
    pub fn new(
        mut sys_ref: GhostActorSystem<'lt, Self, S>,
        owner_seed: GhostEndpointSeed<'lt, KeystoreProtocol, (), S>,
    ) -> GhostResult<Self> {
        let owner_ref = sys_ref.plant_endpoint(
            owner_seed,
            KeystoreActorHandler {
                handle_request_to_actor_sign: Box::new(|_me, _message, cb| {
                    // THIS IS A STUB, just responding with empty signature
                    cb(Ok(SignResultData {
                        signature: b"".to_vec().into(),
                    }))
                }),
            },
        )?;

        Ok(Self {
            _sys_ref: sys_ref,
            _owner_ref: owner_ref,
        })
    }
}

impl<'lt, S: GhostSystemRef<'lt>> GhostActor<'lt, KeystoreProtocol, Keystore2ActorStub<'lt, S>>
    for Keystore2ActorStub<'lt, S>
{
}
