use super::*;

pub type ObjectProperty = (ObjectId, String);

#[derive(Debug, PartialEq)]
pub enum Request {
    Set(ObjectProperty, Decodable),
    Get(ObjectProperty),
    Subscribe(ObjectProperty),
    Unsubscribe(ObjectProperty),
}
