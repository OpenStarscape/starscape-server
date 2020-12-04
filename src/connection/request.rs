use super::*;

pub type EntityProperty = (EntityKey, String);

#[derive(Debug, PartialEq, Clone)]
pub enum PropertyRequest {
    Set(Decoded),
    Get,
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RequestType {
    Property(EntityProperty, PropertyRequest),
    Close,
}

/// An incomming message from a client, only used in the connection module
#[derive(Debug, PartialEq, Clone)]
pub struct Request {
    pub connection: ConnectionKey,
    pub request: RequestType,
}

impl Request {
    pub fn new(connection: ConnectionKey, request: RequestType) -> Self {
        Self {
            connection,
            request,
        }
    }
}
