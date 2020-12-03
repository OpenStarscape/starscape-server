use super::*;

/// Implements both the session and session builder (session builder turns into session when built)
pub struct WebrtcSession {
    dispatcher: WebrtcDispatcher,
    addr: SocketAddr,
    outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, Vec<u8>)>,
}

impl WebrtcSession {
    pub fn new(
        dispatcher: WebrtcDispatcher,
        addr: SocketAddr,
        outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, Vec<u8>)>,
    ) -> Self {
        Self {
            dispatcher,
            addr,
            outbound_tx,
        }
    }
}

impl SessionBuilder for WebrtcSession {
    fn build(
        self: Box<Self>,
        handler: InboundBundleHandler,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        self.dispatcher.set_inbound_handler(&self.addr, handler)?;
        Ok(self)
    }
}

impl Session for WebrtcSession {
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match self.outbound_tx.try_send((self.addr, data.to_vec())) {
            Ok(()) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Err(format!(
                "WebRTC outbound channel is full (can't send bundle to {})",
                self.addr
            )
            .into()),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => Err(format!(
                "WebRTC outbound channel closed (can't send bundle to {})",
                self.addr
            )
            .into()),
        }
    }

    fn max_packet_len(&self) -> usize {
        warn!(
            "returning max WebRTC message length as {}, but in practice it's likely lower",
            webrtc_unreliable::MAX_MESSAGE_LEN
        );
        // TODO: implement this smarter based on what actually works in browsers
        webrtc_unreliable::MAX_MESSAGE_LEN
    }
}

impl Debug for WebrtcSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcSession for {}", self.addr)
    }
}
