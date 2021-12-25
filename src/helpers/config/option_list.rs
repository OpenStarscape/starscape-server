use super::*;

/// These options will be called in order of returned vec (NOT in the order the user specifies the options) if the user
/// specifies them. An option's handler will not be called if is not specified by the user.
pub fn option_list() -> Vec<ConfigOption> {
    vec![
        ConfigOption::new_flag("tcp", |conf, enable| {
            conf.server.tcp = if enable {
                Some(SocketAddrConfig::new_loopback())
            } else {
                None
            };
        }),
        ConfigOption::new_flag("websockets", |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_websockets = enable;
            };
        }),
        ConfigOption::new_flag("webrtc", |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_webrtc_experimental = enable;
            };
        }),
        ConfigOption::new_flag("https", |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.server_type = if enable {
                    HttpServerType::Encrypted {
                        socket_addr: SocketAddrConfig::new_non_loopback(),
                        cert_path: "../ssl/cert.pem".to_string(),
                        key_path: "../ssl/privkey.pem".to_string(),
                        enable_http_to_https_redirect: true,
                    }
                } else {
                    // TODO: stop conflating unencrypted with loopback
                    let mut addr = SocketAddrConfig::new_loopback();
                    addr.port = Some(56_560);
                    HttpServerType::Unencrypted(addr)
                }
            };
        }),
        ConfigOption::new_string("http_content", |conf, path| {
            if let Some(http) = &mut conf.server.http {
                http.static_content_path = Some(path.to_string());
            };
        }),
        ConfigOption::new_parsed(
            "max_game_time",
            |time_str| time_str.parse::<f64>().map_err(Into::into),
            |conf, time| {
                conf.max_game_time = time;
            },
        ),
    ]
}
