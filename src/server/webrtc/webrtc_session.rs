use super::*;

/// Implements both the session and session builder (session builder turns into session when built)
pub struct WebrtcSession {
    dispatcher: WebrtcDispatcher,
    addr: SocketAddr,
    outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, WebrtcMessage)>,
}

impl WebrtcSession {
    pub fn new(
        dispatcher: WebrtcDispatcher,
        addr: SocketAddr,
        outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, WebrtcMessage)>,
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
        handler: Box<dyn InboundBundleHandler>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        self.dispatcher.set_inbound_handler(&self.addr, handler)?;
        // This will send a packet 1 byte longer than Chromium's limit:
        // let data: Vec<u8> = (0..2021).map(|i: u64| (i % 26) as u8 + 97).collect();
        // self.outbound_tx.try_send((self.addr, data));
        Ok(self)
    }
}

impl Session for WebrtcSession {
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if data.len() > self.max_packet_len() {
            warn!(
                "trying to send bundle {} bytes long when WebRTC max packet length is {}",
                data.len(),
                self.max_packet_len()
            );
        }
        match self
            .outbound_tx
            .try_send((self.addr, WebrtcMessage::Data(data.to_vec())))
        {
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

    /// There doesn't seem to be an easy answer for this. [webrtc_unreliable::MAX_MESSAGE_LEN](https://docs.rs/webrtc-unreliable/0.5.0/webrtc_unreliable/constant.MAX_MESSAGE_LEN.html)
    /// exists but the docs for that are basically "probably won't work irl ¯\_(ツ)_/¯". They appear
    /// to be correct. I've tested in a few browsers and this is what I've found:
    /// - Firefox 83.0 on Linux (local Wifi): 9,156
    /// - Chromeium 87.0 on Linux (local Wifi): 2,020
    /// It's further complicated by the fact that if the packet is too big, Firefox seems to
    /// consistently also drop the packet after the next one, and Chromium just *closes the whole
    /// fucking connection*. That is, only if the packet is small enough. If it's *too* big both
    /// browsers will completely ignore it (maybe it got dropped earlier somewhere?). There's some
    /// explanation in this [2016 blogpost](https://lgrahl.de/articles/demystifying-webrtc-dc-size-limit.html),
    /// however that seems to conclude the lowest limit is 16,000, so either browsers have gotten
    /// worse or we're hitting other problems. [This might also be helpful](https://blog.mozilla.org/webrtc/large-data-channel-messages/)
    fn max_packet_len(&self) -> usize {
        2020
    }

    fn close(&mut self) {
        match self.outbound_tx.try_send((self.addr, WebrtcMessage::Close)) {
            Ok(()) => (),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => (),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                error!(
                    "failed to close session {} because send buffer is full",
                    self.addr
                );
                // This just removes it from the list rather than closing it, but probably better
                // than nothing
                self.dispatcher.close_session(&self.addr);
            }
        }
    }
}

impl Debug for WebrtcSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcSession for {}", self.addr)
    }
}
