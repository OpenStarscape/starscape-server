mod connection_impl;
mod connection_trait;
mod request;

pub use connection_impl::ConnectionImpl;
pub use connection_trait::Connection;
pub use request::{ConnectionRequest, ObjectProperty, PropertyRequest, Request};

use super::*;
