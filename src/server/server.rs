use super::*;

const DEFAULT_HTTP_PORT: u16 = 80;
const DEFAULT_HTTPS_PORT: u16 = 443;
const DEFAULT_TCP_PORT: u16 = 56_550;
const DEFAULT_WEB_RTC_PORT: u16 = DEFAULT_TCP_PORT + 1;

/// Represents an object that lives for the lifetime of the server, such as a listener for a
/// particular network protocol
pub trait ServerComponent: Debug {}

/// Creates and owns the various components that allow clients to connect
pub struct Server {
    _components: Vec<Box<dyn ServerComponent>>,
}

impl Server {
    pub fn new(
        config: &ServerConfig,
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut components: Vec<Box<dyn ServerComponent>> = Vec::new();

        if let Some(tcp) = &config.tcp {
            let ip = get_ip(
                tcp.interface_name.as_deref(),
                Some(IpVersion::V4),
                tcp.loopback,
            )?;
            let addr = SocketAddr::new(ip, tcp.port.unwrap_or(DEFAULT_TCP_PORT));
            let tcp = TcpListener::new(new_session_tx.clone(), addr)
                .map_err(|e| format!("failed to create TcpListener: {}", e))?;
            components.push(Box::new(tcp));
        }

        if let Some(http) = &config.http {
            // Is there a simpler way to make an empty warp filter?
            let mut warp_filter = warp::any()
                .and_then(|| async { Err::<Box<dyn warp::Reply>, _>(warp::reject::not_found()) })
                .boxed();

            if http.enable_websockets {
                let (filter, server) = WebsocketServer::new(new_session_tx.clone())
                    .map_err(|e| format!("failed to create WebSocket server: {}", e))?;
                components.push(Box::new(server));
                warp_filter = warp_filter.or(filter).unify().boxed();
            }

            if let Some(webrtc) = &http.webrtc_experimental {
                // Firefox doesn't work when WebRTC is running on a loopback interface. This address is
                // shared automatically by webrtc_unreliable.
                if webrtc.loopback != Some(false) {
                    warn!("non-loopback IP not requested for WebRTC server. If a loopback IP is selected, WebRTC may not work in Firefox");
                }
                let ip = get_ip(
                    webrtc.interface_name.as_deref(),
                    Some(IpVersion::V4),
                    webrtc.loopback,
                )?;
                if ip.is_loopback() {
                    warn!("loopback IP selected for WebRTC server, which may not work in Firefox");
                }
                let addr = SocketAddr::new(ip, webrtc.port.unwrap_or(DEFAULT_WEB_RTC_PORT));
                let (rtc_warp_filter, webrtc) = WebrtcServer::new(addr, new_session_tx)
                    .map_err(|e| format!("failed to create WebrtcServer: {}", e))?;
                components.push(Box::new(webrtc));
                warp_filter = warp_filter.or(rtc_warp_filter).unify().boxed();
            }

            if let Some(static_content_path) = &http.static_content_path {
                let static_content_filter: GenericFilter =
                    warp::fs::dir(static_content_path.to_string())
                        .map(|reply| Box::new(reply) as Box<dyn warp::Reply>)
                        .boxed();
                warp_filter = warp_filter.or(static_content_filter).unify().boxed();
            }

            match &http.server_type {
                HttpServerType::Encrypted {
                    socket_addr: addr,
                    cert_path,
                    key_path,
                    enable_http_to_https_redirect,
                } => {
                    let ip = get_ip(
                        addr.interface_name.as_deref(),
                        Some(IpVersion::V4),
                        addr.loopback,
                    )?;
                    let https_addr = SocketAddr::new(ip, addr.port.unwrap_or(DEFAULT_HTTPS_PORT));
                    let https_server =
                        HttpServer::new_encrypted(warp_filter, https_addr, &cert_path, &key_path)?;
                    components.push(Box::new(https_server));

                    if *enable_http_to_https_redirect {
                        let http_addr = SocketAddr::new(ip, DEFAULT_HTTP_PORT);
                        let http_redirect_server =
                            HttpServer::new_http_to_https_redirect(http_addr)?;
                        components.push(Box::new(http_redirect_server));
                    }
                }
                HttpServerType::Unencrypted(addr) => {
                    // This should resolve to localhost for testing. We need to point the web app to this
                    // address (at time of writing that's done with a proxy rule in vue.config.js).
                    let ip = get_ip(
                        addr.interface_name.as_deref(),
                        Some(IpVersion::V4),
                        addr.loopback,
                    )?;

                    let http_addr = SocketAddr::new(ip, addr.port.unwrap_or(DEFAULT_HTTP_PORT));
                    let http_server = HttpServer::new_unencrypted(warp_filter, http_addr)?;
                    components.push(Box::new(http_server));
                }
            }
        }

        for component in &components {
            info!("{:?}", component);
        }

        Ok(Self {
            _components: components,
        })
    }
}
