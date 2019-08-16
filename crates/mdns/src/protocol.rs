//! Protocol trait definition.

/// Whenever a Multicast DNS responder starts up, wakes up from sleep,
/// receives an indication of a network interface "Link Change" event, or
/// has any other reason to believe that its network connectivity may
/// have changed in some relevant way, it MUST perform the two startup
/// steps below: [Probing](https://tools.ietf.org/html/rfc6762#section-8.1)
/// and [Announcing](https://tools.ietf.org/html/rfc6762#section-8.3).

// use crate::error::MulticastDnsResult;

pub trait Discovery {
    fn startup(&mut self);
    fn update(&mut self);
    fn flush(&mut self);
}
