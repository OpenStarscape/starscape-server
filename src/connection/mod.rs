//! Contains the high level logic for encoding and decoding messages, and managing client
//! connections

use super::*;

#[allow(clippy::module_inception)]
mod connection;
mod connection_collection;
mod decoded;
mod encodable;
mod format;
mod json;
mod object_map;
mod request;

pub use connection::{Connection, ConnectionImpl, ConnectionKey};
pub use connection_collection::{
    ConnectionCollection, InboundMessageHandler, OutboundMessageHandler,
};
pub use decoded::Decoded;
pub use encodable::Encodable;
pub use object_map::{ObjectId, ObjectMap};

use format::{DecodeCtx, Decoder, EncodeCtx, Encoder};
use json::json_protocol_impls;
use object_map::ObjectMapImpl;
use request::{EntityProperty, PropertyRequest, Request, RequestType};
