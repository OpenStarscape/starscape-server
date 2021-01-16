//! # WebRTC session layer implementation
//!
//! WebRTC unreliable data channels are the only way to get data to and from
//! in-browser apps without head-of-line blocking. Implementing a WebRTC
//! server is quite difficult. We use the the
//! [webrtc-unreliable](https://crates.io/crates/webrtc-unreliable) crate wich
//! supports the bare-minimum features to enbable unreliable data channels.

use super::*;

mod webrtc_dispatcher;
mod webrtc_server;
mod webrtc_session;
mod webrtc_warp_filter;

pub use webrtc_server::WebrtcServer;

use webrtc_dispatcher::{WebrtcDispatcher, WebrtcMessage};
use webrtc_session::WebrtcSession;
use webrtc_warp_filter::webrtc_warp_filter;
