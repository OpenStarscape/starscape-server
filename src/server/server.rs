use super::*;

/// Represents an object that lives for the lifetime of the server, such as a listener for a
/// particular network protocol
pub trait ServerComponent: Debug {}

/// Creates and owns the various components that allow clients to connect
pub struct Server {
    _components: Vec<Box<dyn ServerComponent>>,
}

impl Server {
    pub fn new(
        enable_tcp: bool,
        enable_webrtc: bool,
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut components: Vec<Box<dyn ServerComponent>> = Vec::new();

        // Is there a simpler way to make an empty warp filter?
        let mut warp_filter = warp::any()
            .and_then(|| async { Err::<Box<dyn warp::Reply>, _>(warp::reject::not_found()) })
            .boxed();

        if enable_tcp {
            let tcp = TcpListener::new(new_session_tx.clone(), None, None)
                .map_err(|e| format!("failed to create TcpListener: {}", e))?;
            components.push(Box::new(tcp));
        }

        if enable_webrtc {
            // Firefox doesn't work when WebRTC is running on a loopback interface. This address is
            // shared automatically by webrtc_unreliable.
            let addr = SocketAddr::new(get_ip(None, Some(IpVersion::V4), Some(false))?, 42424);
            let (rtc_warp_filter, webrtc) = WebrtcServer::new(addr, new_session_tx)
                .map_err(|e| format!("failed to create WebrtcServer: {}", e))?;
            components.push(Box::new(webrtc));
            warp_filter = warp_filter.or(rtc_warp_filter).unify().boxed();
        }

        {
            // This should resolve to localhost for testing. We need to point the web app to this
            // address (at time of writing that's done with a proxy rule in vue.config.js).
            let addr = SocketAddr::new(get_ip(None, Some(IpVersion::V4), Some(true))?, 56_000);
            let http_server = HttpServer::new(warp_filter, addr)?;
            components.push(Box::new(http_server));
        }

        for component in &components {
            info!("{:?}", component);
        }

        Ok(Self {
            _components: components,
        })
    }
}
