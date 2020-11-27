//! TODO: replace this with the warp crate

use super::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;

async fn handle_upload(
    stream: impl warp::Stream<Item = Result<impl warp::Buf, warp::Error>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(hyper::Body::from("not found")))
}

async fn run(
    endpoint: webrtc_unreliable::SessionEndpoint,
    socket_addr: SocketAddr,
    shutdown_rx: futures::channel::oneshot::Receiver<()>,
) {
    let rtc = warp::post()
        .and(warp::path("rtc"))
        .and(warp::body::stream())
        .and_then(handle_upload);
    /*
        move |request_body| async {
            let a: warp::Stream = request_body;
            Ok::<_, warp::Rejection>(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .body(hyper::Body::from(format!("error"))).expect("???"))
            /*
            match endpoint.http_session_request(request_body).await {
                Ok(mut resp) => {
                    /*trace!(
                        "WebRTC session request from {} got response",
                        remote_addr
                    );*/
                    resp.headers_mut().insert(
                        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        hyper::header::HeaderValue::from_static("*"),
                    );
                    Ok(resp.map(hyper::Body::from))
                }
                Err(err) => {
                    /*warn!(
                        "WebRTC session request from {} got error response",
                        remote_addr
                    );*/
                    Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .body(hyper::Body::from(format!("error: {}", err))).expect("???"))
                }
            }
            */
        }
    );*/

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
                                trace!("WebRTC session request from {}", remote_addr);
                                let foo = req.into_body();
                                match session_endpoint.http_session_request(foo).await {
                                    Ok(mut resp) => {
                                        trace!(
                                            "WebRTC session request from {} got response",
                                            remote_addr
                                        );
                                        resp.headers_mut().insert(
                                            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                            hyper::header::HeaderValue::from_static("*"),
                                        );
                                        Ok(resp.map(hyper::Body::from))
                                    }
                                    Err(err) => {
                                        warn!(
                                            "WebRTC session request from {} got error response",
                                            remote_addr
                                        );
                                        hyper::Response::builder()
                                            .status(hyper::StatusCode::BAD_REQUEST)
                                            .body(hyper::Body::from(format!("error: {}", err)))
                                    }
                                }
                            } else {
                                warn!(
                                    "got invalid {} request to {}",
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
    trace!("WebRTC HTTP server shut down");
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
        let addr = requested_addr.unwrap_or_else(|| Ipv4Addr::LOCALHOST.into());
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
            error!("failed to send WebRTC HTTP server shutdown request");
        };
        if let Err(e) = futures::executor::block_on(self.join_handle.take().unwrap()) {
            error!("failed to join WebRTC HTTP server task: {}", e);
        }
    }
}
