use super::*;

/// The data for a method event. That is, an event for an object member.
#[derive(Debug, PartialEq, Clone)]
pub enum EventMethod {
    /// A response to a get request, or an initial one-time response to a subscribe request.
    Value,
    /// Notify the client of an update to a property they've previously subscribed to.
    Update,
    /// Notify the client that a signal they've subscribed to has fired.
    Signal,
}

/// Represents a message from the server to a client
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    /// A method on an object member (property/action/signal). The member is represented by it's
    /// entity and name).
    Method(GenericId, String, EventMethod, Value),
    /// Notify the client that an object has been destroyed and wont be used any more
    Destroyed(GenericId),
    /// Some problem has caused the server or connection to fail. This should be the last event
    /// before the session is closed. The message should be user-readable.
    FatalError(String),
}

impl Event {
    pub fn value(entity: GenericId, name: String, value: Value) -> Self {
        Self::Method(entity, name, EventMethod::Value, value)
    }

    pub fn update(entity: GenericId, name: String, value: Value) -> Self {
        Self::Method(entity, name, EventMethod::Update, value)
    }

    pub fn signal(entity: GenericId, name: String, value: Value) -> Self {
        Self::Method(entity, name, EventMethod::Signal, value)
    }
}
