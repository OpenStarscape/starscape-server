use super::*;

/// Accepts connections and listens for incoming data on all active connections.
/// Creating a WebrtcListener alone is not enoguh to allow clients to connect.
/// Something must call `.http_session_request()` on the returned session_endpoint.
/// See `WebrtcHttpServer` for an example.
pub struct WebrtcListener {
    webrtc_server: webrtc_unreliable::Server,
    http_server: WebrtcHttpServer,
}

impl WebrtcListener {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(webrtc_unreliable::SessionEndpoint, Self), Box<dyn Error>> {
        let listen_addr = "127.0.0.1:42424".parse()?;
        let public_addr = "127.0.0.1:42424".parse()?;
        let webrtc_server =
            futures::executor::block_on(webrtc_unreliable::Server::new(listen_addr, public_addr))?;
        let session_endpoint = webrtc_server.session_endpoint();
        // TODO: replace this with a warp HTTP server that can be used for other things
        let http_server = WebrtcHttpServer::new(session_endpoint.clone(), None, None)?;
        Ok((
            session_endpoint,
            WebrtcListener {
                webrtc_server,
                http_server,
            },
        ))
    }
}

impl Debug for WebrtcListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcListener") // Not fully implemented
    }
}

impl Listener for WebrtcListener {}
