use super::*;

/// Processes requests from a client. Implemented by State in the engine and used by
/// ConnectionCollection.
pub trait InboundMessageHandler {
    fn set(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: Decoded,
    ) -> Result<(), String>;
    fn get(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
    ) -> Result<Encodable, String>;
    /// If Ok, the returned Any should later be sent to unsubscribe()
    fn subscribe(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
    ) -> Result<Box<dyn Any>, String>;
    fn unsubscribe(&mut self, subscription: Box<dyn Any>) -> Result<(), String>;
}
