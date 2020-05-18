use super::*;

pub type ObjectProperty = (ObjectId, String);

#[derive(Debug, PartialEq)]
pub enum RequestData {
    Set(ObjectProperty, Decodable),
    Get(ObjectProperty),
    Subscribe(ObjectProperty),
    Unsubscribe(ObjectProperty),
    Close,
}

pub struct Request {
    pub connection: ConnectionKey,
    pub data: RequestData,
}

impl Request {
    pub fn new(connection: ConnectionKey, data: RequestData) -> Self {
        Self { connection, data }
    }
}
