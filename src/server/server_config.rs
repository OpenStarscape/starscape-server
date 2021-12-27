/// Parameters to find an IP address and port
pub struct SocketAddrConfig {
    /// The network interface to use such as `wlp3s0`, or None to use any network interface. You can get a list of
    /// interfaces on a Linux computer with `ip link show`
    pub interface_name: Option<String>,
    /// If to get a loopback IP (localhost) or a remote-accessible IP, or None for either.
    pub loopback: Option<bool>,
    /// The port to use, or None for default (default port depends on context, for example it's 80 for HTTP and 443 for
    /// HTTPS)
    pub port: Option<u16>,
}

impl SocketAddrConfig {
    pub fn new_loopback() -> Self {
        Self {
            interface_name: None,
            loopback: Some(true),
            port: None,
        }
    }

    pub fn new_non_loopback() -> Self {
        Self {
            interface_name: None,
            loopback: Some(false),
            port: None,
        }
    }
}

/// Parameters for an encrypted HTTPS server
pub struct HttpsConfig {
    pub socket_addr: SocketAddrConfig,
    /// Path to the certificate (often cert.pem)
    pub cert_path: String,
    /// Path to the private key (often privkey.pem)
    pub key_path: String,
    /// If to spin up an unencrypted HTTP server that redirects to HTTPS. Always runs on the same IP as the
    /// encrypted server and on port 80.
    pub enable_http_to_https_redirect: bool,
}

/// Parameters to create and HTTP or HTTPS server
pub enum HttpServerType {
    /// An unencrypted HTTP server
    Unencrypted(SocketAddrConfig),
    /// An encrypted HTTPS server
    Encrypted(HttpsConfig),
}

/// Parameters for an http server
pub struct HttpServerConfig {
    /// Path to the frontend that will be served
    pub static_content_path: Option<String>,
    /// If to accept websocket connections. It's done through the HTTP server so no additional SocketAddr is required.
    pub enable_websockets: bool,
    ///If to accept WebRTC connections. WARNING: WebRTC is unreliable, and dropped packets are not correctly handled.
    pub enable_webrtc_experimental: bool,
    /// Parameters for the specific server type (encrypted or unencrypted)
    pub server_type: HttpServerType,
}

/// Parameters to create a server
pub struct ServerConfig {
    /// Parameters for TCP listener, or None to disable TCP connections
    pub tcp: Option<SocketAddrConfig>,
    /// Parameters for HTTP/HTTPS server, or None to disable all HTTP-related functionality
    pub http: Option<HttpServerConfig>,
}
