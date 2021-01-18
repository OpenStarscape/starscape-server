use super::*;
use warp::reply::Reply;

/// Uses Warp to spin up an HTTP server. At time of writing this is only used to initialize WebRTC,
/// but it accepts an arbitrary Warp filter and so could easily be used for whatever else we
/// needed.
pub struct HttpServer {
    socket_addr: SocketAddr,
    shutdown_tx: Option<futures::channel::oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl HttpServer {
    #[allow(dead_code)]
    pub fn new_unencrypted(
        filter: GenericFilter,
        socket_addr: SocketAddr,
    ) -> Result<Self, Box<dyn Error>> {
        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel();
        trace!("starting HTTP server on {:?}", socket_addr);
        let (_addr, server) = warp::serve(filter)
            .try_bind_with_graceful_shutdown(socket_addr, async {
                shutdown_rx.await.ok();
            })
            .map_err(|e| format!("failed to bind HTTP server to {}: {}", socket_addr, e))?;
        let join_handle = tokio::spawn(async move {
            server.await;
            trace!("HTTP server shut down");
        });
        Ok(HttpServer {
            socket_addr,
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }

    /// Create a new server that redirects all requests to HTTPS
    pub fn new_https_redirect(socket_addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel();
        trace!("starting redirect-to-HTTPS server on {:?}", socket_addr);
        let (_addr, server) = warp::serve(
            warp::host::optional()
                .and(warp::path::full())
                .and(warp::query::raw())
                .map(
                    |authority: Option<warp::host::Authority>,
                     path: warp::path::FullPath,
                     query: String| {
                        warn!(
                            "redirecting to hard-coded path, request path: {}",
                            path.as_str()
                        );
                        let authority = match authority {
                            Some(a) => a,
                            None => {
								warn!("could not redirect to HTTPS: no authority");
                                return Box::new(
                                    warp::http::status::StatusCode::NOT_FOUND.into_response(),
                                ) as Box<dyn warp::Reply>
                            }
                        };
						let path_and_query_str = if query.is_empty() {
							path.as_str().to_string()
						} else {
							format!("{}?{}", path.as_str(), query)
						};
						let path_and_query = match warp::http::uri::PathAndQuery::from_maybe_shared(path_and_query_str) {
							Ok(p) => p,
							Err(e) => {
								warn!("could not redirect to HTTPS: failed to build path and query: {}", e);
								return Box::new(
                                    warp::http::status::StatusCode::NOT_FOUND.into_response(),
                                ) as Box<dyn warp::Reply>
							}
						};
                        match warp::hyper::Uri::builder()
                            .scheme("https")
                            .authority(authority)
                            .path_and_query(path_and_query)
                            .build()
                        {
                            Ok(uri) => Box::new(warp::redirect(uri)) as Box<dyn warp::Reply>,
                            Err(e) => {
                                error!("could not redirect to HTTPS: failed to build URI: {}", e);
                                Box::new(
                                    warp::http::status::StatusCode::INTERNAL_SERVER_ERROR
                                        .into_response(),
                                ) as Box<dyn warp::Reply>
                            }
                        }
                    },
                ),
        )
        .try_bind_with_graceful_shutdown(socket_addr, async {
            let _ = shutdown_rx.await;
        })
        .map_err(|e| {
            format!(
                "failed to bind HTTP redirect server to {}: {}",
                socket_addr, e
            )
        })?;

        let join_handle = tokio::spawn(async move {
            server.await;
            trace!("HTTPS server shut down");
        });
        Ok(HttpServer {
            socket_addr,
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }

    pub fn new_encrypted(
        filter: GenericFilter,
        socket_addr: SocketAddr,
        cert_path: &str,
        key_path: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel();
        trace!("starting HTTPS server on {:?}", socket_addr);

        let (_addr, server) = warp::serve(filter)
            .tls()
            .cert_path(cert_path)
            .key_path(key_path)
            .bind_with_graceful_shutdown(socket_addr, async {
                let _ = shutdown_rx.await;
            });
        // TODO: we want to use .try_bind_with_graceful_shutdown() (like we do in new_unencrypted())
        // so it doesn't panic if there's an error, but that's not implemented for TlsServer (see
        // https://github.com/seanmonstar/warp/pull/717). Once that PR lands and we upgrade to a
        // warp version that supports it we should use it.

        let join_handle = tokio::spawn(async move {
            server.await;
            trace!("HTTPS server shut down");
        });
        Ok(HttpServer {
            socket_addr,
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        if let Err(()) = self.shutdown_tx.take().unwrap().send(()) {
            error!("failed to send HTTP server shutdown request");
        };
        if let Err(e) = futures::executor::block_on(self.join_handle.take().unwrap()) {
            error!("failed to join HTTP server task: {}", e);
        }
    }
}

impl Debug for HttpServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpServer on {}", self.socket_addr)
    }
}

impl ServerComponent for HttpServer {}
