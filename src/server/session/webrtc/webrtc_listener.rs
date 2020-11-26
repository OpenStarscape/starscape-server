use super::*;

/// Accepts connections and listens for incoming data on all active connections.
/// Creating a WebrtcListener alone is not enoguh to allow clients to connect.
/// Something must call `.http_session_request()` on the returned session_endpoint.
/// See `WebrtcHttpServer` for an example.
pub struct WebrtcListener {
    http_server: WebrtcHttpServer,
}

impl WebrtcListener {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(webrtc_unreliable::SessionEndpoint, Self), Box<dyn Error>> {
        let listen_addr = "192.168.42.232:42424".parse()?;
        let public_addr = "192.168.42.232:42424".parse()?;
        let mut webrtc_server =
            futures::executor::block_on(webrtc_unreliable::Server::new(listen_addr, public_addr))?;
        let session_endpoint = webrtc_server.session_endpoint();
        // TODO: replace this with a warp HTTP server that can be used for other things
        let http_server = WebrtcHttpServer::new(session_endpoint.clone(), None, None)?;

        tokio::spawn(async move {
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
        });

        // There doesn't seem to be a gracefully way to stop the WebRTC server or kill the task, so it is leaked
        // TODO: graceful shutdown

        Ok((session_endpoint, WebrtcListener { http_server }))
    }
}

impl Debug for WebrtcListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcListener") // Not fully implemented
    }
}

impl Listener for WebrtcListener {}
