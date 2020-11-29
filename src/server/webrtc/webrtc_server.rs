use super::*;

/// Runs the WebRTC server loop. As far as I can tell there is no gracefull shutdown, which means
/// this needs to be aborted externally.
async fn run_server(
    dispatcher: WebrtcDispatcher,
    outbound_bundle_rx: Receiver<(SocketAddr, Vec<u8>)>,
    mut webrtc_server: webrtc_unreliable::Server,
) {
    let mut message_buf = Vec::new();
    loop {
        // TODO: abort the server's recv() when an outbound message is ready, or otherwise send it
        match webrtc_server.recv().await {
            Ok(received) => {
                message_buf.clear();
                message_buf.extend(received.message.as_ref());
                dispatcher.dispatch_inbound(received.remote_addr, &message_buf);
            }
            Err(err) => {
                error!("could not receive RTC message: {}", err);
            }
        }

        // doing this loop the obvious `while let Ok((...)) = ...` way leads to some crazy error
        loop {
            let outbound = outbound_bundle_rx.try_recv();
            if let Ok((addr, bundle)) = outbound {
                if let Err(err) = webrtc_server
                    .send(&bundle, webrtc_unreliable::MessageType::Text, &addr)
                    .await
                {
                    warn!("could not send message to {}: {}", addr, err);
                }
            } else {
                break;
            }
        }
    }
}

/// Accepts connections and listens for incoming data on all active connections.
pub struct WebrtcServer {
    listen_addr: SocketAddr,
    abort_handle: Option<future::AbortHandle>,
    join_handle: Option<tokio::task::JoinHandle<Result<(), future::Aborted>>>,
}

impl WebrtcServer {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(GenericFilter, Self), Box<dyn Error>> {
        let listen_addr = "192.168.42.232:42424".parse()?;
        let webrtc_server = block_on(webrtc_unreliable::Server::new(listen_addr, listen_addr))?;
        let endpoint = webrtc_server.session_endpoint();
        let (outbound_bundle_tx, outbound_bundle_rx) = channel();
        let dispatcher = WebrtcDispatcher::new(new_session_tx, outbound_bundle_tx);

        // Use futures::future::Abortable to kill the server on command
        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        let abortable_server = future::Abortable::new(
            run_server(dispatcher, outbound_bundle_rx, webrtc_server),
            abort_registration,
        );
        let join_handle = tokio::spawn(abortable_server);

        Ok((
            webrtc_warp_filter(endpoint),
            Self {
                listen_addr,
                abort_handle: Some(abort_handle),
                join_handle: Some(join_handle),
            },
        ))
    }
}

impl Drop for WebrtcServer {
    fn drop(&mut self) {
        trace!("aborting WebRTC server");
        self.abort_handle.take().unwrap().abort();
        trace!("waiting for WebRTC server to shut down");
        let result = block_on(self.join_handle.take().unwrap());
        trace!("WebRTC server shut down: {:?}", result);
    }
}

impl Debug for WebrtcServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcServer on {:?}", self.listen_addr)
    }
}

impl ServerComponent for WebrtcServer {}
