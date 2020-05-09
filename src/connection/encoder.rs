use std::error::Error;

use super::*;

/// Encodes a specific data format (ex JSON)
/// Any encoder should be compatible with any session (JSON should work with TCP, websockets, etc)
pub trait Encoder {
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}
