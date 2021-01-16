use super::*;

const OUTBOUND_BUNDLE_BUFFER_SIZE: usize = 1000; // max number of in-flight outbound bundles

/// Loops indefinitely waiting for inbound messages. Needs to be aborted externally.
async fn listen_for_inbound(
    dispatcher: &WebrtcDispatcher,
    webrtc_server: &tokio::sync::Mutex<webrtc_unreliable::Server>,
    closed_sessions: &mut HashSet<SocketAddr>,
) {
    let mut webrtc_server = webrtc_server.lock().await;
    let mut message_buf = Vec::new();
    loop {
        match webrtc_server.recv().await {
            Ok(received) => {
                // If we receive a packet from a closed address, assume it has re-connected
                closed_sessions.remove(&received.remote_addr);
                // clearing and filling doesn't require any allocations except when the buffer gets
                // bigger; more efficient than creating a new vector each iteration.
                message_buf.clear();
                message_buf.extend(received.message.as_ref());
                dispatcher.dispatch_inbound(&received.remote_addr, &message_buf);
            }
            Err(err) => {
                // TODO: this is likely fatal, so close or maybe restart the WebRTC server
                error!("could not receive RTC message: {}", err);
            }
        }
    }
}

async fn send_outbound(
    first_outbount: (SocketAddr, WebrtcMessage),
    outbound_rx: &mut tokio::sync::mpsc::Receiver<(SocketAddr, WebrtcMessage)>,
    dispatcher: &WebrtcDispatcher,
    webrtc_server: &tokio::sync::Mutex<webrtc_unreliable::Server>,
    closed_sessions: &mut HashSet<SocketAddr>,
) {
    // Lock the server. This should be uncontested because the listen_for_inbound()
    // should now have been dropped
    let mut webrtc_server = webrtc_server.lock().await;
    // Wrap the outbound message in outbound_rx.try_recv()'s result type, so we can
    // loop over any additional messages when we're done with the initial one
    let mut outbound = Ok(first_outbount);
    while let Ok((addr, message)) = outbound {
        if !closed_sessions.contains(&addr) {
            match message {
                WebrtcMessage::Data(bundle) => {
                    // This actually sends the bundle
                    if let Err(err) = webrtc_server
                        .send(&bundle, webrtc_unreliable::MessageType::Binary, &addr)
                        .await
                    {
                        warn!(
                            "could not send message to {}, closing session: {}",
                            addr, err
                        );
                        webrtc_server.disconnect(&addr);
                        dispatcher.close_session(&addr);
                        closed_sessions.insert(addr);
                    }
                }
                WebrtcMessage::Close => {
                    webrtc_server.disconnect(&addr);
                    dispatcher.close_session(&addr);
                    closed_sessions.insert(addr);
                }
            }
        }
        // If there are multiple outbound messages queued up, processing them now
        // without letting go of the server lock is more efficient than starting
        // and quickly aborting listen_for_inbound() a bunch of times.
        outbound = outbound_rx.try_recv();
    }
}

/// Runs the WebRTC server loop. Needs to be aborted externally.
async fn run_server(
    dispatcher: WebrtcDispatcher,
    mut outbound_rx: tokio::sync::mpsc::Receiver<(SocketAddr, WebrtcMessage)>,
    webrtc_server: webrtc_unreliable::Server,
) {
    let webrtc_server = tokio::sync::Mutex::new(webrtc_server);
    let mut closed_sessions = HashSet::new();
    // Run until we're extenally aborted
    loop {
        // listen for inbound messages (which will run forever) until we have a pending outbound one
        tokio::select! {
            _ = listen_for_inbound(&dispatcher, &webrtc_server, &mut closed_sessions) => (),
            outbound = outbound_rx.recv() => {
                if let Some(outbound) = outbound {
                    send_outbound(
                        outbound,
                        &mut outbound_rx,
                        &dispatcher,
                        &webrtc_server,
                        &mut closed_sessions
                    ).await;
                } else {
                    warn!("outbound bundle sender dropped while WebRTC server was still running");
                }
            },
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
        listen_addr: SocketAddr,
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(GenericFilter, Self), Box<dyn Error>> {
        let webrtc_server = block_on(webrtc_unreliable::Server::new(listen_addr, listen_addr))?;
        let endpoint = webrtc_server.session_endpoint();
        let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel(OUTBOUND_BUNDLE_BUFFER_SIZE);
        let dispatcher = WebrtcDispatcher::new(new_session_tx, outbound_tx);

        // Use futures::future::Abortable to kill the server on command
        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        let abortable_server = future::Abortable::new(
            run_server(dispatcher, outbound_rx, webrtc_server),
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
