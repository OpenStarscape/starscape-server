use super::*;
use warp::{http, reply::Reply};

trait CustomUnwrapResponse {
    fn or_internal_server_error(self) -> Box<dyn warp::Reply>;
}

impl<T: warp::Reply + 'static, E: Error> CustomUnwrapResponse for Result<T, E> {
    fn or_internal_server_error(self) -> Box<dyn warp::Reply> {
        self.map(|ok| -> Box<dyn warp::Reply> { Box::new(ok) })
            .unwrap_or_else(|err| -> Box<dyn warp::Reply> {
                warn!("failed to build WebRTC HTTP response: {}", err);
                Box::new(http::status::StatusCode::INTERNAL_SERVER_ERROR.into_response())
            })
    }
}

/// Responds to an HTTP request, which is the first step in initializing a WebRTC connection. Type
/// signature is figured out with help from https://github.com/seanmonstar/warp/issues/362.
async fn handle_http_request(
    mut endpoint: webrtc_unreliable::SessionEndpoint,
    remote_addr: Option<SocketAddr>,
    stream: impl warp::Stream<Item = Result<impl warp::Buf, warp::Error>>,
) -> Result<Box<dyn warp::Reply>, core::convert::Infallible> {
    // Requires futures::StreamExt to be in scope
    let stream = stream.map(|stream| stream.map(|mut buffer| buffer.to_bytes()));
    match endpoint.session_request(stream).await {
        Ok(body) => {
            // It would be nice to be able to send off a SessionBuilder here, but alas we do not
            // know the address the WebRTC packets will come from, so can not match this request
            // with future packets. Instead, the connection will be created when we get our first
            // packet.
            Ok(http::Response::builder()
                .status(http::status::StatusCode::OK)
                .header(http::header::CONTENT_TYPE, "application/json")
                .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(body)
                .or_internal_server_error())
        }
        Err(err) => {
            warn!("WebRTC request from {:?} got error response", remote_addr);
            Ok(http::Response::builder()
                .status(http::status::StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/plain")
                .body(format!("error: {}", err))
                .or_internal_server_error())
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
