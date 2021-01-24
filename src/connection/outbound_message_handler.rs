use super::*;

/// Allows sending property updates and other messages to clients. Implemented by
/// ConnectionCollection and used by the engine.
pub trait OutboundMessageHandler {
    fn property_update(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
    fn signal(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
}
