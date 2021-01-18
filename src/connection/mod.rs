//! Contains the high level logic for encoding and decoding messages, and managing client
//! connections

use super::*;

#[allow(clippy::module_inception)]
mod connection;
mod connection_collection;
mod decoded;
mod encodable;
mod format;
mod inbound_message_handler;
mod json;
mod object_map;
mod outbound_message_handler;
mod request;

pub use connection::{Connection, ConnectionImpl, ConnectionKey};
pub use connection_collection::ConnectionCollection;
pub use decoded::Decoded;
pub use encodable::Encodable;
pub use inbound_message_handler::InboundMessageHandler;
pub use object_map::{ObjectId, ObjectMap};
pub use outbound_message_handler::OutboundMessageHandler;

use format::{DecodeCtx, Decoder, EncodeCtx, Encoder};
use json::json_protocol_impls;
use object_map::ObjectMapImpl;
use request::{EntityProperty, ObjectRequest, Request, RequestData};
