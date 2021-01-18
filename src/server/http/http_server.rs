use super::*;

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

    pub fn new_encrypted(
        filter: GenericFilter,
        https_socket_addr: SocketAddr,
        http_socket_addr: SocketAddr,
        cert_path: &str,
        key_path: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel();
        let (shutdown_http_tx, shutdown_http_rx) = futures::channel::oneshot::channel();
        trace!("starting HTTPS server on {:?}", https_socket_addr);

        let (_addr, https_server) = warp::serve(filter)
            .tls()
            .cert_path(cert_path)
            .key_path(key_path)
            .bind_with_graceful_shutdown(https_socket_addr, async {
                let _ = shutdown_rx.await;
                let _ = shutdown_http_tx.send(());
            });
        // TODO: we want to use .try_bind_with_graceful_shutdown() (like we do in new_unencrypted())
        // so it doesn't panic if there's an error, but that's not implemented for TlsServer (see
        // https://github.com/seanmonstar/warp/pull/717). Once that PR lands and we upgrade to a
        // warp version that supports it we should use it.

        let (_addr, http_server) =
            warp::serve(warp::path::full().map(|path: warp::path::FullPath| {
                warn!(
                    "redirecting to hard-coded path, request path: {}",
                    path.as_str()
                );
                warp::redirect(warp::hyper::Uri::from_static("https://starscape.wmww.sh"))
            }))
            .try_bind_with_graceful_shutdown(http_socket_addr, async {
                let _ = shutdown_http_rx.await;
            })
            .map_err(|e| {
                format!(
                    "failed to bind HTTP redirect server to {}: {}",
                    http_socket_addr, e
                )
            })?;

        let server = future::join(https_server, http_server);

        let join_handle = tokio::spawn(async move {
            server.await;
            trace!("HTTPS server shut down");
        });
        Ok(HttpServer {
            socket_addr: https_socket_addr,
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
