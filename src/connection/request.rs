use super::*;

/// The data for a method request. That is, a request on an object memeber.
#[derive(Debug, PartialEq, Clone)]
pub enum RequestMethod {
    Action(Decoded),
    Set(Decoded),
    Get,
    Subscribe,
    Unsubscribe,
}

/// Represents a message from a client to the server
#[derive(Debug, PartialEq, Clone)]
pub enum Request {
    /// A method on an object member (property/action/signal). The member is represented by it's
    /// entity and name).
    Method(EntityKey, String, RequestMethod),
    /// Indicates the session should close.
    Close,
}

impl Request {
    pub fn action(entity: EntityKey, name: String, value: Decoded) -> Self {
        Self::Method(entity, name, RequestMethod::Action(value))
    }

    pub fn set(entity: EntityKey, name: String, value: Decoded) -> Self {
        Self::Method(entity, name, RequestMethod::Set(value))
    }

    pub fn get(entity: EntityKey, name: String) -> Self {
        Self::Method(entity, name, RequestMethod::Get)
    }

    pub fn subscribe(entity: EntityKey, name: String) -> Self {
        Self::Method(entity, name, RequestMethod::Subscribe)
    }

    pub fn unsubscribe(entity: EntityKey, name: String) -> Self {
        Self::Method(entity, name, RequestMethod::Unsubscribe)
    }
}
