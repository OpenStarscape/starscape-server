//! Contains the high level logic for encoding and decoding messages, and managing client
//! connections

use super::*;

#[allow(clippy::module_inception)]
mod connection;
mod connection_collection;
mod decode;
mod encode;
mod json;
mod object_map;
mod request;

pub use connection::{Connection, ConnectionImpl, ConnectionKey};
pub use connection_collection::{
    ConnectionCollection, InboundMessageHandler, OutboundMessageHandler,
};
pub use decode::{Decodable, DecodableAs};
pub use encode::Encodable;
pub use object_map::{ObjectId, ObjectMap};

use decode::{DecodeCtx, Decoder};
use encode::{EncodeCtx, Encoder};
use json::json_protocol_impls;
use object_map::ObjectMapImpl;
use request::{ObjectProperty, PropertyRequest, Request, RequestType};

use serde::ser::{Serialize, Serializer};
