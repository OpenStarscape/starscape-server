use super::*;

pub type ObjectProperty = (ObjectId, String);

#[derive(Debug, PartialEq, Clone)]
pub enum PropertyRequest {
    Set(Decodable),
    Get,
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionRequest {
    Property(ObjectProperty, PropertyRequest),
    Close,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ServerRequest {
    pub connection: ConnectionKey,
    pub request: ConnectionRequest,
}

impl ServerRequest {
    pub fn new(connection: ConnectionKey, request: ConnectionRequest) -> Self {
        Self {
            connection,
            request,
        }
    }
}
