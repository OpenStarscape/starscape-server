use super::*;

use std::fmt::Debug;

pub trait InboundBundleHandler: Send {
    fn handle(&mut self, data: &[u8]);
}

/// A client that is trying to connect
pub trait SessionBuilder: Send + Debug {
    /// Try to build the session, handle_packet will receive any packets that
    /// have already arrived, plus all future packets.
    fn build(
        self: Box<Self>,
        handler: Box<dyn InboundBundleHandler>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>>;
}

/// Represents a low-level network connection. Abstracts over things like Unix
/// sockets, UDP, TCP and WebRTC data channels (not all of these are implemented
/// at this time). Reliable+ordered session types use the same data format as
/// unreliable+unordered.
pub trait Session: Send + Debug {
    /// Sends a bundle of data to the client. Bundles should be assumed to be unreliable+unordered.
    /// This errors if there's an issue with the underlying connection, or if data is longer
    /// than max_packet_len() has ever been.
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
    /// The longest a packet should be. This may change (if, for example, long
    /// packets are frequently dropped). It should be avoided, but It's not an
    /// error to send a packet with a previously-allowed length (this would be
    /// impossible to prevent in a thread-safe way).
    fn max_packet_len(&self) -> usize;
}
