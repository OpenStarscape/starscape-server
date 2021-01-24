//! Contains the high level logic for encoding and decoding messages, and managing client
//! connections

use super::*;

mod bundle_handler;
#[allow(clippy::module_inception)]
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
pub use event::Event;
pub use message_handlers::{EventHandler, RequestHandler};
pub use object_map::{ObjectId, ObjectMap};
pub use request_error::{RequestError, RequestError::*, RequestResult};

use bundle_handler::BundleHandler;
use event::EventMethod;
use format::{DecodeCtx, Decoder, EncodeCtx, Encoder};
use json::json_protocol_impls;
use object_map::ObjectMapImpl;
use request::{Request, RequestMethod};
