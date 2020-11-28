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
    pub fn new(
        filter: GenericFilter,
        address: Option<IpAddr>,
        port: Option<u16>,
    ) -> Result<Self, Box<dyn Error>> {
        let address = address.unwrap_or_else(|| Ipv4Addr::LOCALHOST.into());
        let port = port.unwrap_or(80);
        let socket_addr = SocketAddr::new(address, port);
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
