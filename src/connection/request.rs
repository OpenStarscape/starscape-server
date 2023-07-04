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
#[derive(PartialEq, Clone)]
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

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::Method(id, name, method) => match method {
                RequestMethod::Action(value) => write!(f, "{:?}.{}({:?})", id, name, value),
                RequestMethod::Set(value) => write!(f, "{:?}.{} = {:?}", id, name, value),
                RequestMethod::Get => write!(f, "{:?}.get {}", id, name),
                RequestMethod::Subscribe => write!(f, "{:?}.subscribe to {}", id, name),
                RequestMethod::Unsubscribe => write!(f, "{:?}.unsubscribe from {}", id, name),
            },
            Self::Close => write!(f, "CLOSED"),
        }
    }
}
