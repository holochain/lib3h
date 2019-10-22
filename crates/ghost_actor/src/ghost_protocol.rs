use crate::*;
use holochain_tracing::*;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Is this message directed toward an actor or an owner
pub enum GhostProtocolDestination {
    Actor,
    Owner,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The type of message. Responses are directed toward the original requestor
pub enum GhostProtocolVariantType {
    Event,
    Request,
    Response,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Identifies the *type* of variant, without considering the content
pub struct GhostProtocolDiscriminant {
    pub id: &'static str,
    pub destination: GhostProtocolDestination,
    pub variant_type: GhostProtocolVariantType,
}

impl GhostProtocolDiscriminant {
    /// debug friendly identifier for the protocol variant type
    pub fn id(&self) -> &str {
        &self.id
    }

    /// whether this message variant type is directed to the actor or owner
    pub fn destination(&self) -> &GhostProtocolDestination {
        &self.destination
    }

    /// whether this message variant type is an event, request, or response
    pub fn variant_type(&self) -> &GhostProtocolVariantType {
        &self.variant_type
    }
}

/// A GhostProtocol is most often an enum, the variants of which contain
/// some additional meta-information indicating the message direction / type.
pub trait GhostProtocol: 'static + Debug + Clone + Send + Sync {
    /// a list of all variant types associated with this protocol
    fn discriminant_list() -> &'static [GhostProtocolDiscriminant];

    /// the meta information for this actual variant instance
    fn discriminant(&self) -> &GhostProtocolDiscriminant;

    // -- provided -- //

    /// is this message destined for the actor?
    fn is_to_actor(&self) -> bool {
        match self.discriminant().destination() {
            GhostProtocolDestination::Actor => true,
            GhostProtocolDestination::Owner => false,
        }
    }

    /// is this message destined for the owner?
    fn is_to_owner(&self) -> bool {
        !self.is_to_actor()
    }

    /// is this an event not expecting a response?
    fn is_event(&self) -> bool {
        if let GhostProtocolVariantType::Event = self.discriminant().variant_type() {
            true
        } else {
            false
        }
    }

    /// is this a request that expects a response?
    fn is_request(&self) -> bool {
        if let GhostProtocolVariantType::Request = self.discriminant().variant_type() {
            true
        } else {
            false
        }
    }

    /// is this a response to a previous request?
    fn is_response(&self) -> bool {
        if let GhostProtocolVariantType::Response = self.discriminant().variant_type() {
            true
        } else {
            false
        }
    }
}

/// when a protocol handler received a callback for fulfilling a request
/// it will follow this signature
pub type GhostHandlerCb<'lt, T> = Box<dyn FnOnce(Span, T) -> GhostResult<()> + 'lt + Send + Sync>;

/// any protocol handler should implement this trait
pub trait GhostHandler<'lt, X: 'lt + Send + Sync, P: GhostProtocol>: Send + Sync {
    fn trigger(
        &mut self,
        _span: Span,
        user_data: &mut X,
        message: P,
        cb: Option<GhostHandlerCb<'lt, P>>,
    ) -> GhostResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    pub mod proto {
        use super::*;

        #[derive(Debug, Clone, PartialEq)]
        pub struct Print(pub String);
        #[derive(Debug, Clone, PartialEq)]
        pub struct Add1(pub i32);

        #[derive(Debug, Clone, PartialEq)]
        pub enum Protocol {
            EventToActorPrint(Print),
            RequestToActorAdd1(Add1),
        }

        static D_LIST: &'static [GhostProtocolDiscriminant] = &[
            GhostProtocolDiscriminant {
                id: "event_to_actor_print",
                destination: GhostProtocolDestination::Actor,
                variant_type: GhostProtocolVariantType::Event,
            },
            GhostProtocolDiscriminant {
                id: "request_to_actor_add_1",
                destination: GhostProtocolDestination::Actor,
                variant_type: GhostProtocolVariantType::Request,
            },
        ];

        impl GhostProtocol for Protocol {
            fn discriminant_list() -> &'static [GhostProtocolDiscriminant] {
                D_LIST
            }

            fn discriminant(&self) -> &GhostProtocolDiscriminant {
                match self {
                    Protocol::EventToActorPrint(_) => &D_LIST[0],
                    Protocol::RequestToActorAdd1(_) => &D_LIST[1],
                }
            }
        }
    }

    use proto::*;

    #[test]
    fn it_should_equate() {
        let a = Protocol::EventToActorPrint(Print("test1".to_string()));
        let a1 = Protocol::EventToActorPrint(Print("test1".to_string()));
        let b = Protocol::EventToActorPrint(Print("test2".to_string()));
        let c = Protocol::RequestToActorAdd1(Add1(42));

        assert_eq!(a, a1);
        assert_ne!(a, b);
        assert_eq!(a.discriminant(), b.discriminant());
        assert_ne!(a.discriminant(), c.discriminant());
    }
}
