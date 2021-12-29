use super::*;

/// Parameters to find an IP address and port
#[derive(Debug)]
pub struct SocketAddrConfig {
    /// The network interface to use such as `wlp3s0`, or None to use any network interface. You can get a list of
    /// interfaces on a Linux computer with `ip link show`
    pub interface_name: Option<String>,
    /// If to get a loopback IP (localhost) or a remote-accessible IP, or None for either.
    pub loopback: Option<bool>,
    /// The port to use
    pub port: u16,
}

impl Default for SocketAddrConfig {
    fn default() -> Self {
        Self {
            interface_name: None,
            loopback: None,
            port: 0,
        }
    }
}

/// Parameters for an encrypted HTTPS server
#[derive(Debug)]
pub struct EncryptedHttpsConfig {
    pub socket_addr: SocketAddrConfig,
    /// Path to the certificate (often cert.pem)
    pub cert_path: String,
    /// Path to the private key (often privkey.pem)
    pub key_path: String,
    /// If to spin up an unencrypted HTTP server that redirects to HTTPS. Always runs on the same IP as the
    /// encrypted server and on port 80.
    pub enable_http_to_https_redirect: bool,
}

impl Default for EncryptedHttpsConfig {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddrConfig::default(),
            cert_path: String::new(),
            key_path: String::new(),
            enable_http_to_https_redirect: false,
        }
    }
}

/// Parameters to create and HTTP or HTTPS server
#[derive(Debug)]
pub enum HttpServerType {
    /// An unencrypted HTTP server
    Unencrypted(SocketAddrConfig),
    /// An encrypted HTTPS server
    Encrypted(EncryptedHttpsConfig),
}

/// Parameters for an http server
#[derive(Debug)]
pub struct HttpServerConfig {
    /// Path to the frontend that will be served
    pub static_content_path: Option<String>,
    /// If to attempt to start a web browser locally
    pub open_browser: bool,
    /// If to accept websocket connections. It's done through the HTTP server so no additional SocketAddr is required.
    pub enable_websockets: bool,
    ///If to accept WebRTC connections. WARNING: WebRTC is unreliable, and dropped packets are not correctly handled.
    pub enable_webrtc_experimental: bool,
    /// Parameters for the specific server type (encrypted or unencrypted)
    pub server_type: HttpServerType,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            static_content_path: None,
            open_browser: false,
            enable_websockets: false,
            enable_webrtc_experimental: false,
            server_type: HttpServerType::Unencrypted(SocketAddrConfig::default()),
        }
    }
}

/// Parameters to create a server
#[derive(Debug)]
pub struct ServerConfig {
    /// Parameters for TCP listener, or None to disable TCP connections
    pub tcp: Option<SocketAddrConfig>,
    /// Parameters for HTTP/HTTPS server, or None to disable all HTTP-related functionality
    pub http: Option<HttpServerConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tcp: None,
            http: None,
        }
    }
}

fn warn_component_disabled(source: Option<&str>, component: &str) {
    if let Some(source) = source {
        // If this is *not* the default value
        warn!("{} ignored because {} is disabled", source, component);
    }
}

fn with_tcp_conf<F>(
    conf: &mut MasterConfig,
    source: Option<&str>,
    cb: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&mut SocketAddrConfig) -> Result<(), Box<dyn Error>>,
{
    if let Some(tcp) = &mut conf.server.tcp {
        cb(tcp)
    } else {
        warn_component_disabled(source, "TCP");
        Ok(())
    }
}

fn with_http_conf<F>(
    conf: &mut MasterConfig,
    source: Option<&str>,
    cb: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&mut HttpServerConfig) -> Result<(), Box<dyn Error>>,
{
    if let Some(http) = &mut conf.server.http {
        cb(http)
    } else {
        warn_component_disabled(source, "HTTP server");
        Ok(())
    }
}

fn with_encrypted_conf<F>(
    conf: &mut MasterConfig,
    source: Option<&str>,
    cb: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&mut EncryptedHttpsConfig) -> Result<(), Box<dyn Error>>,
{
    with_http_conf(conf, source, |http| {
        if let HttpServerType::Encrypted(https) = &mut http.server_type {
            cb(https)
        } else {
            warn_component_disabled(source, "HTTPS encryption");
            Ok(())
        }
    })
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
                conf.server.tcp = match enable {
                    true => Some(SocketAddrConfig::default()),
                    false => None,
                };
                Ok(())
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "tcp_loopback",
            "use localhost for TCP instead of an externally accessible IP",
            false,
            |conf, enable, source| {
                with_tcp_conf(conf, source, |tcp| {
                    tcp.loopback = Some(enable);
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_int(
            "tcp_port",
            "TCP listener port",
            DEFAULT_TCP_PORT as i64,
            move |conf, port, source| {
                let port = u16::try_from(port)
                    .map_err(|_| format!("{} set to invalid port {}", source.unwrap(), port))?;
                with_tcp_conf(conf, source, |tcp| {
                    tcp.port = port;
                    Ok(())
                })
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
                        conf.server.http = Some({
                            let mut http = HttpServerConfig::default();
                            http.server_type =
                                HttpServerType::Unencrypted(SocketAddrConfig::default());
                            http
                        });
                    },
                ),
                <dyn ConfigEntry>::new_enum_variant(
                    "https",
                    "encrypted HTTPS server",
                    |conf, _| {
                        conf.server.http = Some({
                            let mut http = HttpServerConfig::default();
                            http.server_type =
                                HttpServerType::Encrypted(EncryptedHttpsConfig::default());
                            http
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
            "http_loopback",
            "use localhost for HTTP(S) instead of an externally accessible IP",
            false,
            |conf, enable, source| {
                with_http_conf(conf, source, |http| {
                    match &mut http.server_type {
                        HttpServerType::Unencrypted(socket_addr) => {
                            socket_addr.loopback = Some(enable)
                        }
                        HttpServerType::Encrypted(https) => {
                            https.socket_addr.loopback = Some(enable)
                        }
                    }
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_int(
            "http_port",
            &format!(
                "HTTP(S) server port (defaults to {} instead of {} if HTTPS is used)",
                DEFAULT_HTTPS_PORT, DEFAULT_HTTP_PORT
            ),
            DEFAULT_HTTP_PORT as i64,
            move |conf, port, source| {
                let port = u16::try_from(port)
                    .map_err(|_| format!("{} set to invalid port {}", source.unwrap(), port))?;
                with_http_conf(conf, source, |http| {
                    match &mut http.server_type {
                        HttpServerType::Unencrypted(socket_addr) => socket_addr.port = port,
                        HttpServerType::Encrypted(https) => {
                            if source.is_none() {
                                // If this is the default value, use the HTTPS port instead of the HTTP one
                                assert_eq!(port, DEFAULT_HTTP_PORT);
                                https.socket_addr.port = DEFAULT_HTTPS_PORT;
                            } else {
                                // Otherwise, accept the port the user gave
                                https.socket_addr.port = port;
                            }
                        }
                    }
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_string(
            "http_static_content",
            concat!(
                "path to the directory of static content to be served over HTTP",
                " (such as the web frontent public/ directory)"
            ),
            "",
            |conf, path, source| {
                with_http_conf(conf, source, |http| {
                    http.static_content_path = if path.len() > 0 {
                        Some(path.to_string())
                    } else {
                        None
                    };
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "open_browser",
            "attempt to launch the default web browser locally to connect to the HTTP server",
            false,
            |conf, enable, source| {
                with_http_conf(conf, source, |http| {
                    http.open_browser = enable;
                    if enable && http.static_content_path.is_none() {
                        warn_component_disabled(source, "HTTP static content");
                    }
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "enable_websockets",
            "accept/reject WebSocket connections",
            true,
            |conf, enable, source| {
                with_http_conf(conf, source, |http| {
                    http.enable_websockets = enable;
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "enable_webrtc_experimental",
            concat!(
                "accept/reject WebRTC sessions.",
                " WARNING: WebRTC sessions may experience bugs due to dropped packets"
            ),
            false,
            |conf, enable, source| {
                with_http_conf(conf, source, |http| {
                    http.enable_webrtc_experimental = enable;
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_string(
            "https_cert_path",
            "path to the certificate used for HTTPS",
            "cert.pem",
            |conf, path, source| {
                with_encrypted_conf(conf, source, |https| {
                    https.cert_path = path.clone();
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_string(
            "https_key_path",
            "path to the private key used for HTTPS",
            "privkey.pem",
            |conf, path, source| {
                with_encrypted_conf(conf, source, |https| {
                    https.key_path = path.clone();
                    Ok(())
                })
            },
        ),
        <dyn ConfigEntry>::new_bool(
            "redirect_http_to_https",
            "run an HTTP server that redirects to the HTTPS server",
            true,
            |conf, enable, source| {
                with_encrypted_conf(conf, source, |https| {
                    https.enable_http_to_https_redirect = enable;
                    Ok(())
                })
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_disabled_by_default() {
        let fs = MockFilesystem::new();
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(conf.server.tcp.is_none());
    }

    #[test]
    fn can_enable_tcp() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(conf.server.tcp.is_some());
    }

    #[test]
    fn tcp_loopback_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().loopback, Some(false));
    }

    #[test]
    fn can_enable_tcp_loopback() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "enable_tcp = true\ntcp_loopback = true");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().loopback, Some(true));
    }

    #[test]
    fn tcp_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().port, DEFAULT_TCP_PORT);
    }

    #[test]
    fn can_set_tcp_port() {
        let fs =
            MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true\ntcp_port = 99");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().port, 99);
    }

    #[test]
    fn invalid_tcp_port_results_in_config_error() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "enable_tcp = true; tcp_port = 100000");
        assert!(build_config_with(vec![], fs.boxed()).is_err());
    }

    #[test]
    fn can_use_encrypted_https_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(_) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn can_use_unencrypted_http_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"http\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(_) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn can_disable_http_server() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"none\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(conf.server.http.is_none());
    }

    #[test]
    fn http_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"http\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.port, 80),
            _ => panic!(),
        };
    }

    #[test]
    fn https_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.port, 443),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_http_port() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "http_type = \"http\"\nhttp_port = 44");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.port, 44),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_https_port() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "http_type = \"https\"\nhttp_port = 55");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.port, 55),
            _ => panic!(),
        };
    }

    #[test]
    fn http_loopback_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"http\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.loopback, Some(false)),
            _ => panic!(),
        };
    }

    #[test]
    fn https_loopback_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.loopback, Some(false)),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_http_loopback() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"http\"\nhttp_loopback = true",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.loopback, Some(true)),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_https_loopback() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nhttp_loopback = true",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.loopback, Some(true)),
            _ => panic!(),
        };
    }

    #[test]
    fn websockets_enabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(conf.server.http.unwrap().enable_websockets);
    }

    #[test]
    fn can_disable_websockets() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nenable_websockets = false",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(!conf.server.http.unwrap().enable_websockets);
    }

    #[test]
    fn webrtc_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(!conf.server.http.unwrap().enable_webrtc_experimental);
    }

    #[test]
    fn can_enable_webrtc() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nenable_webrtc_experimental = true",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert!(conf.server.http.unwrap().enable_webrtc_experimental);
    }

    #[test]
    fn default_cert_and_key_paths() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => {
                assert!(https.cert_path.contains("cert"));
                assert!(https.key_path.contains("key"));
            }
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_cert_and_key_paths() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nhttps_cert_path = \"foo.txt\"\nhttps_key_path = \"bar.txt\"",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => {
                assert_eq!(https.cert_path, "foo.txt");
                assert_eq!(https.key_path, "bar.txt");
            }
            _ => panic!(),
        };
    }

    #[test]
    fn http_to_https_redirect_on_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => {
                assert!(https.enable_http_to_https_redirect);
            }
            _ => panic!(),
        };
    }

    #[test]
    fn http_to_https_redirect_can_be_disabled() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nredirect_http_to_https = false",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => {
                assert!(!https.enable_http_to_https_redirect);
            }
            _ => panic!(),
        };
    }

    #[test]
    fn static_content_none_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.http.unwrap().static_content_path, None);
    }

    #[test]
    fn static_content_can_be_set() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nhttp_static_content = \"../foo\"",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(
            conf.server.http.unwrap().static_content_path,
            Some("../foo".to_owned())
        );
    }

    #[test]
    fn setting_static_content_to_empty_string_makes_it_none() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nhttp_static_content = \"\"",
        );
        let conf = build_config_with(vec![], fs.boxed()).unwrap();
        assert_eq!(conf.server.http.unwrap().static_content_path, None);
    }
}
