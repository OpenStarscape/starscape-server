use super::*;

/// Implements both the session and session builder (session builder turns into session when built)
pub struct WebrtcSession {
    dispatcher: WebrtcDispatcher,
    addr: SocketAddr,
    bundle_tx: Sender<(SocketAddr, Vec<u8>)>,
}

impl WebrtcSession {
    pub fn new(
        dispatcher: WebrtcDispatcher,
        addr: SocketAddr,
        bundle_tx: Sender<(SocketAddr, Vec<u8>)>,
    ) -> Self {
        Self {
            dispatcher,
            addr,
            bundle_tx,
        }
    }
}

impl SessionBuilder for WebrtcSession {
    fn build(
        self: Box<Self>,
        mut handler: InboundBundleHandler,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        self.dispatcher.set_inbound_handler(self.addr, handler)?;
        Ok(self)
    }
}

impl Session for WebrtcSession {
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.bundle_tx.send((self.addr, data.to_vec()))?;
        Ok(())
    }

    fn max_packet_len(&self) -> usize {
        warn!(
            "returning max WebRTC message length as {}, but in practice it's likely lower",
            webrtc_unreliable::MAX_MESSAGE_LEN
        );
        webrtc_unreliable::MAX_MESSAGE_LEN
    }
}

impl Debug for WebrtcSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcSession for {}", self.addr)
    }
}
