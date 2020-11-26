use super::*;

async fn run_server(mut webrtc_server: webrtc_unreliable::Server) {
    let mut message_buf = Vec::new();
    loop {
        let received = match webrtc_server.recv().await {
            Ok(received) => {
                message_buf.clear();
                message_buf.extend(received.message.as_ref());
                eprintln!("Received {} bytes of data", received.message.len());
                Some((received.message_type, received.remote_addr))
            }
            Err(err) => {
                eprintln!("Could not receive RTC message: {}", err);
                None
            }
        };

        if let Some((message_type, remote_addr)) = received {
            if let Err(err) = webrtc_server
                .send(&message_buf, message_type, &remote_addr)
                .await
            {
                eprintln!("could not send message to {}: {}", remote_addr, err);
            }
        }
    }
}

/// Accepts connections and listens for incoming data on all active connections.
pub struct WebrtcListener {
    http_server: WebrtcHttpServer,
    abort_handle: Option<future::AbortHandle>,
    join_handle: Option<tokio::task::JoinHandle<Result<(), future::Aborted>>>,
}

impl WebrtcListener {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(webrtc_unreliable::SessionEndpoint, Self), Box<dyn Error>> {
        let listen_addr = "192.168.42.232:42424".parse()?;
        let public_addr = "192.168.42.232:42424".parse()?;
        let webrtc_server = block_on(webrtc_unreliable::Server::new(listen_addr, public_addr))?;
        let session_endpoint = webrtc_server.session_endpoint();
        // TODO: replace this with a warp HTTP server that can be used for other things
        let http_server = WebrtcHttpServer::new(session_endpoint.clone(), None, None)?;
        // Use futures::future::Abortable to kill the server on command
        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        let abortable_server =
            future::Abortable::new(run_server(webrtc_server), abort_registration);
        let join_handle = tokio::spawn(abortable_server);
        Ok((
            session_endpoint,
            WebrtcListener {
                http_server,
                abort_handle: Some(abort_handle),
                join_handle: Some(join_handle),
            },
        ))
    }
}

impl Drop for WebrtcListener {
    fn drop(&mut self) {
        eprintln!("Aborting WebRTC server");
        self.abort_handle.take().unwrap().abort();
        eprintln!("Waiting for WebRTC server to shut down");
        let result = block_on(self.join_handle.take().unwrap());
        eprintln!("WebRTC server shut down: {:?}", result);
    }
}

impl Debug for WebrtcListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcListener") // Not fully implemented
    }
}

impl Listener for WebrtcListener {}
