//! Contains the high level logic for encoding and decoding messages, and managing client
//! connections

use super::*;

mod bundle_handler;
mod connection;
mod connection_collection;
mod event;
mod format;
mod json;
mod message_handlers;
mod object_map;
mod request;
mod request_error;

pub use connection::{Connection, ConnectionImpl, ConnectionKey};
pub use connection_collection::ConnectionCollection;
pub use event::{Event, EventMethod};
pub use message_handlers::{EventHandler, RequestHandler, Subscription};
pub use object_map::{new_encode_ctx, ObjectId, ObjectMap};
pub use request::{Request, RequestMethod};
pub use request_error::{RequestError, RequestError::*, RequestResult};

use bundle_handler::BundleHandler;
use format::{DecodeCtx, Decoder, EncodeCtx, Encoder};
use json::json_protocol_impls;
use object_map::ObjectMapImpl;
