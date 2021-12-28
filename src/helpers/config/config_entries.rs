use super::*;

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

/// These entries will be applied in order of returned vec (NOT in the order the user specifies the entry). All entries
/// Will always be applied.
pub fn config_entries() -> Vec<Box<dyn ConfigEntry>> {
    // We concat! long strings so the vec can be formatted by rustfmt (see https://github.com/rust-lang/rustfmt/issues/3863)
    vec![
        <dyn ConfigEntry>::new_float(
            "max_game_seconds",
            "seconds to run the game before exiting, or 0 to run until process is killed",
            60.0 * 60.0,
            |conf, time, source| {
                if time > 0.0 {
                    conf.max_game_time = Some(time);
                } else {
                    if time < 0.0 {
                        warn!("{} should not be negative", source.unwrap());
                    }
                    conf.max_game_time = None;
                }
            },
        ),
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
                ConfigEntryVariant::new("http", "unencrypted HTTP server", |conf, _| {
                    conf.server.http = Some(HttpServerConfig {
                        static_content_path: None,
                        enable_websockets: false,
                        enable_webrtc_experimental: false,
                        server_type: HttpServerType::Unencrypted(
                            SocketAddrConfig::new_non_loopback(),
                        ),
                    });
                }),
                ConfigEntryVariant::new("https", "encrypted HTTPS server", |conf, _| {
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
                }),
                ConfigEntryVariant::new("none", "do not spin up an HTTP server", |conf, _| {
                    conf.server.http = None;
                }),
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
