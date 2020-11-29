use super::*;

/// Responds to an HTTP request, which is the first step in initializing a WebRTC connection. Type
/// signature is figured out with help from https://github.com/seanmonstar/warp/issues/362.
async fn handle_http_request(
    mut endpoint: webrtc_unreliable::SessionEndpoint,
    remote_addr: Option<SocketAddr>,
    stream: impl warp::Stream<Item = Result<impl warp::Buf, warp::Error>>,
) -> Result<Box<dyn warp::Reply>, core::convert::Infallible> {
    // Requires futures::StreamExt to be in scope
    let stream = stream.map(|stream| stream.map(|mut buffer| buffer.to_bytes()));
    match endpoint.http_session_request(stream).await {
        Ok(mut resp) => {
            // It would be nice to be able to send off a SessionBuilder here, but alas we do not
            // know the address the WebRTC packets will come from, so can not match this request
            // with future packets. Instead, the connection will be created when we get our first
            // packet.
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

/// Returns a warp::Filter that, when added to a Warp HTTP server, initiates WebRTC connections
pub fn webrtc_warp_filter(endpoint: webrtc_unreliable::SessionEndpoint) -> GenericFilter {
    warp::path("rtc")
        .and(warp::post())
        .and(warp::addr::remote())
        .and(warp::body::stream())
        .and_then(move |remote_addr, request_body| {
            handle_http_request(endpoint.clone(), remote_addr, request_body)
        })
        .boxed()
}
