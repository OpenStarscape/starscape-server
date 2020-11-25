//! TODO: replace this with the warp crate

use super::*;
use std::net::{IpAddr, SocketAddr};

async fn run(
    endpoint: webrtc_unreliable::SessionEndpoint,
    socket_addr: SocketAddr,
    shutdown_rx: futures::channel::oneshot::Receiver<()>,
) {
    let make_svc =
        hyper::service::make_service_fn(move |addr_stream: &hyper::server::conn::AddrStream| {
            let endpoint = endpoint.clone();
            let remote_addr = addr_stream.remote_addr();
            async move {
                Ok::<_, hyper::Error>(hyper::service::service_fn(
                    move |req: hyper::Request<hyper::Body>| {
                        let mut session_endpoint = endpoint.clone();
                        async move {
                            if req.uri().path() == "/rtc" && req.method() == hyper::Method::POST {
                                eprintln!("WebRTC session request from {}", remote_addr);
                                match session_endpoint.http_session_request(req.into_body()).await {
                                    Ok(mut resp) => {
                                        resp.headers_mut().insert(
                                            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                            hyper::header::HeaderValue::from_static("*"),
                                        );
                                        Ok(resp.map(hyper::Body::from))
                                    }
                                    Err(err) => hyper::Response::builder()
                                        .status(hyper::StatusCode::BAD_REQUEST)
                                        .body(hyper::Body::from(format!("error: {}", err))),
                                }
                            } else {
                                eprintln!(
                                    "Got invalid {} request to {}",
                                    req.method(),
                                    req.uri().path()
                                );
                                hyper::Response::builder()
                                    .status(hyper::StatusCode::NOT_FOUND)
                                    .body(hyper::Body::from("not found"))
                            }
                        }
                    },
                ))
            }
        });

    let server = hyper::Server::try_bind(&socket_addr)
        .expect("failed to bind to socket")
        .serve(make_svc)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        });
    server.await.expect("server failed");
    eprintln!("WebRTC HTTP server shut down");
}

pub struct WebrtcHttpServer {
    shutdown_tx: Option<futures::channel::oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebrtcHttpServer {
    pub fn new(
        endpoint: webrtc_unreliable::SessionEndpoint,
        requested_addr: Option<IpAddr>,
        requested_port: Option<u16>,
    ) -> Result<Self, Box<dyn Error>> {
        let addr = requested_addr.unwrap_or("::1".parse()?);
        let port = requested_port.unwrap_or(56_000);
        let socket_addr = SocketAddr::new(addr, port);

        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel();

        let join_handle = tokio::spawn(run(endpoint, socket_addr, shutdown_rx));

        Ok(WebrtcHttpServer {
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }
}

impl Drop for WebrtcHttpServer {
    fn drop(&mut self) {
        if let Err(()) = self.shutdown_tx.take().unwrap().send(()) {
            eprintln!("Failed to send WebRTC HTTP server shutdown request");
        };
        if let Err(e) = futures::executor::block_on(self.join_handle.take().unwrap()) {
            eprintln!("Failed to join WebRTC HTTP server task: {}", e);
        }
    }
}
