//! Traits used for encoding and decoding data
use super::*;

/// The required context for encoding. The normal implementation is ObjectMapImpl.
pub trait EncodeCtx {
    /// Returns the object ID for the given entity, creating a new one if needed
    fn object_for(&self, entity: EntityKey) -> ObjectId;
}

/// Encodes a specific data format (ex JSON)
/// Any encoder should be compatible with any session (JSON should work with TCP, websockets, etc)
pub trait Encoder {
    /// An update to a subscribed property resulting from a change
    fn encode_property_update(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    /// A response to a clients get requst on a property
    fn encode_get_response(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    /// A signal sent from the server to clients
    fn encode_signal(
        &self,
        object: ObjectId,
        property: &str,
        ctx: &dyn EncodeCtx,
        value: &Encodable,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    /// An error detected by the server
    fn encode_error(&self, text: &str) -> Result<Vec<u8>, Box<dyn Error>>;
}

/// The context required for decoding a Decoded. The normal implementation is ObjectMapImpl.
pub trait DecodeCtx: Send + Sync {
    /// Returns the entity for the given object ID, or Err if it does not exist
    fn entity_for(&self, object: ObjectId) -> Result<EntityKey, String>;
}

/// Decodes a stream of bytes from the session into requests
pub trait Decoder: Send {
    fn decode(
        &mut self,
        ctx: &dyn DecodeCtx,
        bytes: Vec<u8>,
    ) -> Result<Vec<Request>, Box<dyn Error>>;
}
