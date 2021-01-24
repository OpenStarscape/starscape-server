use super::*;

/// An action on a property, signal or action of an object
#[derive(Debug, PartialEq, Clone)]
pub enum ObjectRequest {
    Set(Decoded),
    Get,
    Subscribe,
    Unsubscribe,
}

/// Represents a message from a client to the server
#[derive(Debug, PartialEq, Clone)]
pub enum Request {
    /// A request on an object (represented by an entity). The String is the member name.
    Object(EntityKey, String, ObjectRequest),
    /// Indicates the session should close.
    Close,
}
