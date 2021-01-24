use super::*;

/// Allows sending property updates and other messages to clients. Implemented by
/// ConnectionCollection and used by the engine.
pub trait OutboundMessageHandler {
    fn event(&self, connection: ConnectionKey, event: Event);
}
