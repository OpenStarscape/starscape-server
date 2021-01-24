use super::*;

/// Processes requests from a client. Implemented by State in the engine and used by
/// ConnectionCollection.
pub trait RequestHandler {
    fn fire_action(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
        value: Value,
    ) -> RequestResult<()>;
    fn set_property(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
        value: Value,
    ) -> RequestResult<()>;
    fn get_property(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
    ) -> RequestResult<Value>;
    /// If Ok, the returned Any should later be sent to unsubscribe(). The name may refer to either
    /// a property or a signal.
    fn subscribe(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
    ) -> RequestResult<Box<dyn Any>>;
    /// Takes a subscription that was previously returned from subscribe()
    fn unsubscribe(&mut self, subscription: Box<dyn Any>) -> RequestResult<()>;
}

/// Allows sending property updates and other messages to clients. Implemented by
/// ConnectionCollection and used by the engine.
pub trait EventHandler {
    fn event(&self, connection: ConnectionKey, event: Event);
}
