use super::*;

#[allow(clippy::module_inception)]
mod connection;
mod json;
mod encode;
mod decode;
mod object_map;
mod request;

pub use connection::{Connection, ConnectionImpl, ConnectionKey};
pub use decode::{Decodable, DecodableAs};
pub use encode::Encodable;
pub use object_map::{ObjectId, ObjectMap};
pub use request::{ConnectionRequest, ObjectProperty, PropertyRequest, ServerRequest};

use decode::{DecodeCtx, Decoder};
use encode::{EncodeCtx, Encoder};
use object_map::ObjectMapImpl;
use json::json_protocol_impls;

use serde::ser::{Serialize, Serializer};
