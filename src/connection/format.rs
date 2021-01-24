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
    /// Encode an event
    fn encode_event(&self, ctx: &dyn EncodeCtx, event: &Event) -> Result<Vec<u8>, Box<dyn Error>>;
}

/// The context required for decoding a Value. The normal implementation is ObjectMapImpl.
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
