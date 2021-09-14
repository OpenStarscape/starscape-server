use super::*;

use std::fmt::Debug;

pub trait InboundBundleHandler: Send {
    fn handle(&mut self, data: &[u8]);
    fn close(&mut self);
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

/// Represents a reliable, ordered data connection to a client. Abstracts over things like TCP
/// connection and WebSockets.
pub trait Session: Send + Debug {
    /// Sends the given data to the client. This errors if there's an issue with the underlying
    /// connection, or if data is longer than max_packet_len().
    /// TODO: this should take an Arc<[u8]> so it can be sent on channels without being copied
    fn send_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
    /// The longest a packet should be, should always return the same value for the same session.
    fn max_packet_len(&self) -> usize;
    /// Close the session, which should result in its inbound handler getting a close() (although
    /// not necessarily immediately)
    fn close(&mut self);
}
