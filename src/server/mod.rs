//! The lower level network code, including the session layer for supported network protocols

use super::*;

mod http;
#[allow(clippy::module_inception)]
mod server;
mod session;
mod tcp;
mod webrtc;

pub use server::Server;
pub use session::{Session, SessionBuilder, InboundBundleHandler};

use http::*;
use server::ServerComponent;
use tcp::*;
use webrtc::*;

type GenericFilter = warp::filters::BoxedFilter<(Box<dyn warp::Reply>,)>;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;
