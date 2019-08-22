//! Protocol trait definition.

/// Whenever a Multicast DNS responder starts up, wakes up from sleep,
/// receives an indication of a network interface "Link Change" event, or
/// has any other reason to believe that its network connectivity may
/// have changed in some relevant way, it MUST perform the two startup
/// steps below: [Probing](https://tools.ietf.org/html/rfc6762#section-8.1)
/// and [Announcing](https://tools.ietf.org/html/rfc6762#section-8.3).
use crate::error::MulticastDnsResult;
// use std::io::Error as IOError;
// pub type Result<T> = std::result::Result<T, IOError>;

pub trait Discovery {
    fn advertise(&mut self);
    fn discover(&mut self) -> MulticastDnsResult<()>;
    fn release(&mut self);
    fn flush(&mut self);
}
