use super::*;

pub type EntityProperty = (EntityKey, String);

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectRequest {
    Set(Decoded),
    Get,
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RequestData {
    Object(EntityProperty, ObjectRequest),
    Close,
}

/// An incomming message from a client, only used in the connection module
#[derive(Debug, PartialEq, Clone)]
pub struct Request {
    pub connection: ConnectionKey,
    pub data: RequestData,
}

impl Request {
    pub fn new(connection: ConnectionKey, data: RequestData) -> Self {
        Self { connection, data }
    }

    #[allow(dead_code)]
    pub fn new_object_request(
        connection: ConnectionKey,
        entity: EntityKey,
        property: String,
        data: ObjectRequest,
    ) -> Self {
        Self::new(connection, RequestData::Object((entity, property), data))
    }

    pub fn new_close_request(connection: ConnectionKey) -> Self {
        Self::new(connection, RequestData::Close)
    }
}
