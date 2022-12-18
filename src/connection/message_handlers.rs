use super::*;

pub trait Subscription: Send + Sync {
    fn finalize(self: Box<Self>, handler: &dyn RequestHandler) -> RequestResult<()>;
}

/// Processes requests from a client. Implemented by State in the engine and used by
/// ConnectionCollection.
pub trait RequestHandler: AsRef<dyn Any> {
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
    /// a property or a signal. If the name is None, the entity's destruction signal is subscribed.
    fn subscribe(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: Option<&str>,
    ) -> RequestResult<Box<dyn Subscription>>;
}

/// Allows sending property updates and other messages to clients. Implemented by
/// ConnectionCollection and used by the engine.
pub trait EventHandler {
    fn event(&self, connection: ConnectionKey, event: Event);
}
