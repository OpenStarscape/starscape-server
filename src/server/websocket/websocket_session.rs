use super::*;
use futures::StreamExt;

const OUTBOUND_BUNDLE_BUFFER_SIZE: usize = 1000; // max number of in-flight outbound bundles

async fn send(
    outbound_tx: &mut futures::stream::SplitSink<warp::ws::WebSocket, warp::ws::Message>,
    outbound_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    if let Err(e) = outbound_rx
        .map(|packet| Ok(warp::ws::Message::binary(packet)))
        .forward(outbound_tx)
        .await
    {
        warn!("sending packet: {}", e);
    }
}

async fn receive(
    inbound_rx: &mut futures::stream::SplitStream<warp::ws::WebSocket>,
    mut handler: Box<dyn InboundBundleHandler>,
) {
    while let Some(result) = inbound_rx.next().await {
        match result {
            Ok(message) => {
                if message.is_text() || message.is_binary() {
                    handler.handle(message.as_bytes());
                }
            }
            Err(e) => warn!("receiving packet: {}", e),
        }
    }
    // Socket has been closed from the client side
    handler.close();
}

async fn run_websocket(
    websocket: warp::ws::WebSocket,
    outbound_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    handler: Box<dyn InboundBundleHandler>,
) {
    let (mut tx, mut rx) = websocket.split();
    tokio::join!(send(&mut tx, outbound_rx), receive(&mut rx, handler));
    let result = tx.reunite(rx);
    match result {
        Ok(websocket) => {
            if let Err(e) = websocket.close().await {
                error!("closing WebSocket: {}", e);
            }
        }
        Err(e) => error!("reuniting WebSocket: {}", e),
    }
}

pub struct WebsocketSessionBuilder {
    websocket: warp::ws::WebSocket,
}

impl WebsocketSessionBuilder {
    pub fn new(websocket: warp::ws::WebSocket) -> Self {
        Self { websocket }
    }
}

impl Debug for WebsocketSessionBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebsocketSessionBuilder")
    }
}

impl SessionBuilder for WebsocketSessionBuilder {
    fn build(
        self: Box<Self>,
        handler: Box<dyn InboundBundleHandler>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel(OUTBOUND_BUNDLE_BUFFER_SIZE);
        tokio::spawn(run_websocket(self.websocket, outbound_rx, handler));
        Ok(Box::new(WebsocketSession { outbound_tx }))
    }
}

pub struct WebsocketSession {
    outbound_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl Session for WebsocketSession {
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match self.outbound_tx.try_send(data.to_vec()) {
            Ok(()) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                Err("WebSocket outbound channel is full (can't send bundle)".into())
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                Err("WebSocket outbound channel closed (can't send bundle)".into())
            }
        }
    }

    fn max_packet_len(&self) -> usize {
        std::usize::MAX
    }
}

impl Debug for WebsocketSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebsocketSession")
    }
}
