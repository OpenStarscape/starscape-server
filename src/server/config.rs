use super::*;

/// Versions of IP address
#[derive(Debug, Clone, Copy)]
pub enum IpVersion {
    V4,
    V6,
}

/// Configuration required to choose an IP address to bind to
pub struct IpConfig {
	/// The network interface name to use, or None if it does not matter
	interface_name: Option<String>,
	/// The version of IP address to use, or None if it does not matter
    version: Option<IpVersion>,
	/// If the IP address should be a loopback device (localhost), or None if it does not matter
    loopback: Option<bool>,
}

pub struct SocketAddrConfig {
	port: u16,
	ip: IpConfig,
}

/// Configuration required to set up an HTTPS server
pub struct TlsConfig {
	port: u16,
	socket_addr: SocketAddrConfig,
    cert_path: String,
    key_path: String,
}

pub enum HttpType {
	Unencrypted{port: u16},
	Tls(TlsConfig),
}

pub struct ServerConfig {
	tcp_port: Option<u16>,
	websockets_port: Option<u16>,
	webrtc_port: Option<u16>,
	tls_enabled: 
	static_content_path: Option<String>,
}
