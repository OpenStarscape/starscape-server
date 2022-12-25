use super::*;

/// The data for a method request. That is, a request on an object memeber.
#[derive(Debug, PartialEq, Clone)]
pub enum RequestMethod {
    Action(Value),
    Set(Value),
    Get,
    Subscribe,
    Unsubscribe,
}

/// Represents a message from a client to the server
#[derive(Debug, PartialEq, Clone)]
pub enum Request {
    /// A method on an object member (property/action/signal). The member is represented by it's
    /// entity and name).
    Method(GenericId, String, RequestMethod),
    /// Indicates the session should close.
    Close,
}

impl Request {
    pub fn action(id: GenericId, name: String, value: Value) -> Self {
        Self::Method(id, name, RequestMethod::Action(value))
    }

    pub fn set(id: GenericId, name: String, value: Value) -> Self {
        Self::Method(id, name, RequestMethod::Set(value))
    }

    pub fn get(id: GenericId, name: String) -> Self {
        Self::Method(id, name, RequestMethod::Get)
    }

    pub fn subscribe(id: GenericId, name: String) -> Self {
        Self::Method(id, name, RequestMethod::Subscribe)
    }

    pub fn unsubscribe(id: GenericId, name: String) -> Self {
        Self::Method(id, name, RequestMethod::Unsubscribe)
    }
}
