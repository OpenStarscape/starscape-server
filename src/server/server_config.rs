use super::*;

/// Parameters to find an IP address and port
#[derive(Debug)]
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
#[derive(Debug)]
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
#[derive(Debug)]
pub enum HttpServerType {
    /// An unencrypted HTTP server
    Unencrypted(SocketAddrConfig),
    /// An encrypted HTTPS server
    Encrypted(HttpsConfig),
}

/// Parameters for an http server
#[derive(Debug)]
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
#[derive(Debug)]
pub struct ServerConfig {
    /// Parameters for TCP listener, or None to disable TCP connections
    pub tcp: Option<SocketAddrConfig>,
    /// Parameters for HTTP/HTTPS server, or None to disable all HTTP-related functionality
    pub http: Option<HttpServerConfig>,
}

fn warn_disabled_http(source: Option<&str>) {
    if let Some(source) = source {
        warn!("{} ignored because HTTP server is disabled", source);
    }
}

fn warn_disabled_http_encryption(source: Option<&str>) {
    if let Some(source) = source {
        warn!("{} ignored because HTTPS encryption is disabled", source);
    }
}

/// Applied in order of returned vec (NOT in the order the user specifies the entry). All entries are always be applied.
pub fn server_config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    vec![
        <dyn ConfigEntry>::new_bool(
            "enable_tcp",
            "accept/reject TCP sessions",
            false,
            |conf, enable, _| {
                conf.server.tcp = if enable {
                    Some(SocketAddrConfig::new_loopback())
                } else {
                    None
                };
            },
        ),
        <dyn ConfigEntry>::new_enum(
            "http_type",
            concat!(
                "type of HTTP server to spin up",
                " (used for WebSockets, WebRTC and serving the web frontend)"
            ),
            vec![
                <dyn ConfigEntry>::new_enum_variant(
                    "http",
                    "unencrypted HTTP server",
                    |conf, _| {
                        conf.server.http = Some(HttpServerConfig {
                            static_content_path: None,
                            enable_websockets: false,
                            enable_webrtc_experimental: false,
                            server_type: HttpServerType::Unencrypted(
                                SocketAddrConfig::new_non_loopback(),
                            ),
                        });
                    },
                ),
                <dyn ConfigEntry>::new_enum_variant(
                    "https",
                    "encrypted HTTPS server",
                    |conf, _| {
                        conf.server.http = Some(HttpServerConfig {
                            static_content_path: None,
                            enable_websockets: false,
                            enable_webrtc_experimental: false,
                            server_type: HttpServerType::Encrypted(HttpsConfig {
                                socket_addr: SocketAddrConfig::new_non_loopback(),
                                cert_path: String::new(),
                                key_path: String::new(),
                                enable_http_to_https_redirect: true,
                            }),
                        });
                    },
                ),
                <dyn ConfigEntry>::new_enum_variant(
                    "none",
                    "do not spin up an HTTP server",
                    |conf, _| {
                        conf.server.http = None;
                    },
                ),
            ],
        ),
        <dyn ConfigEntry>::new_bool(
            "websockets",
            "accept/reject WebSocket connections",
            true,
            |conf, enable, source| {
                if let Some(http) = &mut conf.server.http {
                    http.enable_websockets = enable;
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "webrtc",
            concat!(
                "accept/reject WebRTC sessions.",
                " WARNING: WebRTC sessions may experience bugs due to dropped packets"
            ),
            false,
            |conf, enable, source| {
                if let Some(http) = &mut conf.server.http {
                    http.enable_webrtc_experimental = enable;
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "https",
            "enable/disable encryption on the server",
            false,
            |conf, enable, source| {
                if source.is_none() {
                    return;
                }
                if let Some(http) = &mut conf.server.http {
                    http.server_type = if enable {
                        HttpServerType::Encrypted(HttpsConfig {
                            socket_addr: SocketAddrConfig::new_non_loopback(),
                            cert_path: String::new(),
                            key_path: String::new(),
                            enable_http_to_https_redirect: true,
                        })
                    } else {
                        // TODO: stop conflating unencrypted with loopback
                        let mut addr = SocketAddrConfig::new_loopback();
                        addr.port = Some(56_560);
                        HttpServerType::Unencrypted(addr)
                    }
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
        <dyn ConfigEntry>::new_string(
            "https_cert_path",
            "path to the certificate used for HTTPS",
            "../ssl/cert.pem",
            |conf, path, source| {
                if let Some(http) = &mut conf.server.http {
                    if let HttpServerType::Encrypted(https) = &mut http.server_type {
                        https.cert_path = path;
                    } else {
                        warn_disabled_http_encryption(source);
                    }
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
        <dyn ConfigEntry>::new_string(
            "https_key_path",
            "path to the private key used for HTTPS",
            "../ssl/privkey.pem",
            |conf, path, source| {
                if let Some(http) = &mut conf.server.http {
                    if let HttpServerType::Encrypted(https) = &mut http.server_type {
                        https.key_path = path;
                    } else {
                        warn_disabled_http_encryption(source);
                    }
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
        <dyn ConfigEntry>::new_string(
            "http_content",
            concat!(
                "path to the directory where static content for the HTTP server is stored",
                " (such as the web frontent public/ directory)"
            ),
            "",
            |conf, path, source| {
                if let Some(http) = &mut conf.server.http {
                    http.static_content_path = if path.len() > 0 {
                        Some(path.to_string())
                    } else {
                        None
                    };
                } else {
                    warn_disabled_http(source);
                }
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_use_encrypted_https_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(_) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn can_use_unencrypted_http_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"http\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(_) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn can_disable_http_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"none\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(conf.server.http.is_none());
    }

    #[test]
    fn default_cert_and_key_paths() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "https = true");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => {
                assert!(https.cert_path.contains("cert"));
                assert!(https.key_path.contains("key"));
            }
            _ => panic!("config has unencrypted server"),
        };
    }
}
