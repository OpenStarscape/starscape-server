use super::*;

// type signature is figured out with help from https://github.com/seanmonstar/warp/issues/362
async fn handle_http_request(
    mut endpoint: webrtc_unreliable::SessionEndpoint,
    remote_addr: Option<SocketAddr>,
    stream: impl warp::Stream<Item = Result<impl warp::Buf, warp::Error>>,
) -> Result<Box<dyn warp::Reply>, core::convert::Infallible> {
    let stream = stream.map(|stream| {
        stream.map(|mut buffer| {
            let bytes = buffer.to_bytes();
            warn!("bytes: {:?}", bytes);
            bytes
        })
    });
    match endpoint.http_session_request(stream).await {
        Ok(mut resp) => {
            trace!("WebRTC request from {:?} got response", remote_addr);
            resp.headers_mut().insert(
                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                hyper::header::HeaderValue::from_static("*"),
            );
            Ok(Box::new(resp.map(hyper::Body::from)))
        }
        Err(err) => {
            warn!("WebRTC request from {:?} got error response", remote_addr);
            Ok(Box::new(
                hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .body(hyper::Body::from(format!("error: {}", err)))
                    .expect("failed to build BAD_REQUEST response"),
            ))
        }
    }
}

async fn run_server(mut webrtc_server: webrtc_unreliable::Server) {
    let mut message_buf = Vec::new();
    loop {
        let received = match webrtc_server.recv().await {
            Ok(received) => {
                message_buf.clear();
                message_buf.extend(received.message.as_ref());
                trace!("received {} bytes of data", received.message.len());
                Some((received.message_type, received.remote_addr))
            }
            Err(err) => {
                error!("could not receive RTC message: {}", err);
                None
            }
        };

        if let Some((message_type, remote_addr)) = received {
            if let Err(err) = webrtc_server
                .send(&message_buf, message_type, &remote_addr)
                .await
            {
                error!("could not send message to {}: {}", remote_addr, err);
            }
        }
    }
}

/// Accepts connections and listens for incoming data on all active connections.
pub struct WebrtcListener {
    listen_addr: SocketAddr,
    abort_handle: Option<future::AbortHandle>,
    join_handle: Option<tokio::task::JoinHandle<Result<(), future::Aborted>>>,
}

impl WebrtcListener {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(GenericFilter, Self), Box<dyn Error>> {
        let listen_addr = "192.168.42.232:42424".parse()?;
        let webrtc_server = block_on(webrtc_unreliable::Server::new(listen_addr, listen_addr))?;
        let endpoint = webrtc_server.session_endpoint();

        let warp_filter = warp::path("rtc")
            .and(warp::post())
            .and(warp::addr::remote())
            .and(warp::body::stream())
            .and_then(move |remote_addr, request_body| {
                handle_http_request(endpoint.clone(), remote_addr, request_body)
            })
            .boxed();

        // Use futures::future::Abortable to kill the server on command
        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        let abortable_server =
            future::Abortable::new(run_server(webrtc_server), abort_registration);
        let join_handle = tokio::spawn(abortable_server);

        Ok((
            warp_filter,
            WebrtcListener {
                listen_addr,
                abort_handle: Some(abort_handle),
                join_handle: Some(join_handle),
            },
        ))
    }
}

impl Drop for WebrtcListener {
    fn drop(&mut self) {
        trace!("aborting WebRTC server");
        self.abort_handle.take().unwrap().abort();
        trace!("waiting for WebRTC server to shut down");
        let result = block_on(self.join_handle.take().unwrap());
        trace!("WebRTC server shut down: {:?}", result);
    }
}

impl Debug for WebrtcListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcListener on {:?}", self.listen_addr)
    }
}

impl ServerComponent for WebrtcListener {}
