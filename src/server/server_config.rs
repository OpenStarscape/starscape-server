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

impl Default for HttpsConfig {
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

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            static_content_path: None,
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

fn ip_entries<F>(
    prefix: &str,
    loopback_help: &str,
    port_help: &str,
    default_port: u16,
    callback: F,
) -> Vec<Box<dyn ConfigEntry>>
where
    F: Fn(&mut MasterConfig, Option<&str>, &dyn Fn(&mut SocketAddrConfig)) + 'static,
{
    let callback = Arc::new(callback);
    vec![
        <dyn ConfigEntry>::new_bool(&format!("{}_loopback", prefix), loopback_help, false, {
            let callback = callback.clone();
            move |conf, enable, source| {
                callback(conf, source, &|addr| {
                    addr.loopback = Some(enable);
                });
                Ok(())
            }
        }),
        <dyn ConfigEntry>::new_int(
            &format!("{}_port", prefix),
            port_help,
            default_port as i64,
            move |conf, port, source| {
                let port = u16::try_from(port)
                    .map_err(|_| format!("{} set to invalid port {}", source.unwrap(), port))?;
                callback(conf, source, &|addr| {
                    addr.port = port;
                });
                Ok(())
            },
        ),
    ]
}

/// Applied in order of returned vec (NOT in the order the user specifies the entry). All entries are always be applied.
pub fn server_config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // Pushing to a mutable vec gets us better code formatting than using vec![] and .chain()
    let mut entries = Vec::new();
    entries.push(<dyn ConfigEntry>::new_bool(
        "enable_tcp",
        "accept/reject TCP sessions",
        false,
        |conf, enable, _| {
            conf.server.tcp = if enable {
                Some(SocketAddrConfig::default())
            } else {
                None
            };
            Ok(())
        },
    ));
    entries.append(&mut ip_entries(
        "tcp",
        "use loopback or externally accessible IP for TCP",
        "TCP port",
        DEFAULT_TCP_PORT,
        |conf, source, callback| {
            if let Some(tcp) = &mut conf.server.tcp {
                callback(tcp);
            } else {
                warn_component_disabled(source, "TCP");
            }
        },
    ));
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    entries.push(<dyn ConfigEntry>::new_enum(
        "http_type",
        concat!(
            "type of HTTP server to spin up",
            " (used for WebSockets, WebRTC and serving the web frontend)"
        ),
        vec![
            <dyn ConfigEntry>::new_enum_variant("http", "unencrypted HTTP server", |conf, _| {
                conf.server.http = Some({
                    let mut http = HttpServerConfig::default();
                    http.server_type = HttpServerType::Unencrypted(SocketAddrConfig::default());
                    http
                });
            }),
            <dyn ConfigEntry>::new_enum_variant("https", "encrypted HTTPS server", |conf, _| {
                conf.server.http = Some({
                    let mut http = HttpServerConfig::default();
                    http.server_type = HttpServerType::Encrypted(HttpsConfig::default());
                    http
                });
            }),
            <dyn ConfigEntry>::new_enum_variant(
                "none",
                "do not spin up an HTTP server",
                |conf, _| {
                    conf.server.http = None;
                },
            ),
        ],
    ));
    entries.push(<dyn ConfigEntry>::new_bool(
        "enable_websockets",
        "accept/reject WebSocket connections",
        true,
        |conf, enable, source| {
            if let Some(http) = &mut conf.server.http {
                http.enable_websockets = enable;
            } else {
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries.push(<dyn ConfigEntry>::new_bool(
        "enable_webrtc_experimental",
        concat!(
            "accept/reject WebRTC sessions.",
            " WARNING: WebRTC sessions may experience bugs due to dropped packets"
        ),
        false,
        |conf, enable, source| {
            if let Some(http) = &mut conf.server.http {
                http.enable_webrtc_experimental = enable;
            } else {
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries.append(&mut ip_entries(
        "http",
        "use loopback or externally accessible IP for HTTP(S)",
        &format!(
            "HTTP(S) port (defaults to {} instead of {} when using HTTPS)",
            DEFAULT_HTTPS_PORT, DEFAULT_HTTP_PORT
        ),
        DEFAULT_HTTP_PORT,
        |conf, source, callback| {
            if let Some(http) = &mut conf.server.http {
                match &mut http.server_type {
                    HttpServerType::Unencrypted(socket_addr) => callback(socket_addr),
                    HttpServerType::Encrypted(https) => {
                        if source.is_none() {
                            // if this is the default value
                            https.socket_addr.port = DEFAULT_HTTPS_PORT;
                        } else {
                            callback(&mut https.socket_addr);
                        }
                    }
                }
            } else {
                warn_component_disabled(source, "HTTP server");
            }
        },
    ));
    entries.push(<dyn ConfigEntry>::new_string(
        "https_cert_path",
        "path to the certificate used for HTTPS",
        "cert.pem",
        |conf, path, source| {
            if let Some(http) = &mut conf.server.http {
                if let HttpServerType::Encrypted(https) = &mut http.server_type {
                    https.cert_path = path;
                } else {
                    warn_component_disabled(source, "HTTPS encryption");
                }
            } else {
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries.push(<dyn ConfigEntry>::new_string(
        "https_key_path",
        "path to the private key used for HTTPS",
        "privkey.pem",
        |conf, path, source| {
            if let Some(http) = &mut conf.server.http {
                if let HttpServerType::Encrypted(https) = &mut http.server_type {
                    https.key_path = path;
                } else {
                    warn_component_disabled(source, "HTTPS encryption");
                }
            } else {
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries.push(<dyn ConfigEntry>::new_string(
        "http_static_content",
        concat!(
            "path to the directory of static content to be served over HTTP",
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
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries.push(<dyn ConfigEntry>::new_bool(
        "redirect_http_to_https",
        "run an HTTP server that redirects to the HTTPS server",
        true,
        |conf, enable, source| {
            if let Some(http) = &mut conf.server.http {
                if let HttpServerType::Encrypted(https) = &mut http.server_type {
                    https.enable_http_to_https_redirect = enable;
                } else {
                    warn_component_disabled(source, "HTTPS encryption");
                }
            } else {
                warn_component_disabled(source, "HTTP server");
            }
            Ok(())
        },
    ));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_disabled_by_default() {
        let fs = MockFilesystem::new();
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(conf.server.tcp.is_none());
    }

    #[test]
    fn can_enable_tcp() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(conf.server.tcp.is_some());
    }

    #[test]
    fn tcp_loopback_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().loopback, Some(false));
    }

    #[test]
    fn can_enable_tcp_loopback() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "enable_tcp = true\ntcp_loopback = true");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().loopback, Some(true));
    }

    #[test]
    fn tcp_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().port, DEFAULT_TCP_PORT);
    }

    #[test]
    fn can_set_tcp_port() {
        let fs =
            MockFilesystem::new().add_file("starscape.toml", "enable_tcp = true\ntcp_port = 99");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.tcp.unwrap().port, 99);
    }

    #[test]
    fn invalid_tcp_port_results_in_config_error() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "enable_tcp = true; tcp_port = 100000");
        assert!(build_config_with(fs.boxed()).is_err());
    }

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
    fn http_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"http\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.port, 80),
            _ => panic!(),
        };
    }

    #[test]
    fn https_uses_correct_port_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.port, 443),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_http_port() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "http_type = \"http\"\nhttp_port = 44");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Unencrypted(addr) => assert_eq!(addr.port, 44),
            _ => panic!(),
        };
    }

    #[test]
    fn can_set_https_port() {
        let fs = MockFilesystem::new()
            .add_file("starscape.toml", "http_type = \"https\"\nhttp_port = 55");
        let conf = build_config_with(fs.boxed()).unwrap();
        match conf.server.http.unwrap().server_type {
            HttpServerType::Encrypted(https) => assert_eq!(https.socket_addr.port, 55),
            _ => panic!(),
        };
    }

    #[test]
    fn websockets_enabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(conf.server.http.unwrap().enable_websockets);
    }

    #[test]
    fn can_disable_websockets() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nenable_websockets = false",
        );
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(!conf.server.http.unwrap().enable_websockets);
    }

    #[test]
    fn webrtc_disabled_by_default() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(!conf.server.http.unwrap().enable_webrtc_experimental);
    }

    #[test]
    fn can_enable_webrtc() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nenable_webrtc_experimental = true",
        );
        let conf = build_config_with(fs.boxed()).unwrap();
        assert!(conf.server.http.unwrap().enable_webrtc_experimental);
    }

    #[test]
    fn default_cert_and_key_paths() {
        let fs = MockFilesystem::new().add_file("starscape.toml", "http_type = \"https\"");
        let conf = build_config_with(fs.boxed()).unwrap();
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
        let conf = build_config_with(fs.boxed()).unwrap();
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
        let conf = build_config_with(fs.boxed()).unwrap();
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
        let conf = build_config_with(fs.boxed()).unwrap();
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
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.http.unwrap().static_content_path, None);
    }

    #[test]
    fn static_content_can_be_set() {
        let fs = MockFilesystem::new().add_file(
            "starscape.toml",
            "http_type = \"https\"\nhttp_static_content = \"../foo\"",
        );
        let conf = build_config_with(fs.boxed()).unwrap();
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
        let conf = build_config_with(fs.boxed()).unwrap();
        assert_eq!(conf.server.http.unwrap().static_content_path, None);
    }
}
