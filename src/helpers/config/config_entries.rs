use super::*;

/// These entries will be applied in order of returned vec (NOT in the order the user specifies the entry). All entries
/// Will always be applied.
pub fn config_entries() -> Vec<Box<dyn ConfigEntry>> {
    vec![
        <dyn ConfigEntry>::new_bool("tcp", false, |conf, enable| {
            conf.server.tcp = if enable {
                Some(SocketAddrConfig::new_loopback())
            } else {
                None
            };
        }),
        <dyn ConfigEntry>::new_bool("websockets", true, |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_websockets = enable;
            };
        }),
        <dyn ConfigEntry>::new_bool("webrtc", false, |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_webrtc_experimental = enable;
            };
        }),
        <dyn ConfigEntry>::new_bool("https", false, |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.server_type = if enable {
                    HttpServerType::Encrypted(HttpsConfig {
                        socket_addr: SocketAddrConfig::new_non_loopback(),
                        cert_path: "../ssl/cert.pem".to_string(),
                        key_path: "../ssl/privkey.pem".to_string(),
                        enable_http_to_https_redirect: true,
                    })
                } else {
                    // TODO: stop conflating unencrypted with loopback
                    let mut addr = SocketAddrConfig::new_loopback();
                    addr.port = Some(56_560);
                    HttpServerType::Unencrypted(addr)
                }
            };
        }),
        <dyn ConfigEntry>::new_string("http_content", "", |conf, path| {
            if let Some(http) = &mut conf.server.http {
                http.static_content_path = if path.len() > 0 {
                    Some(path.to_string())
                } else {
                    None
                };
            };
        }),
        <dyn ConfigEntry>::new_float("max_game_time", 60.0 * 60.0, |conf, time| {
            conf.max_game_time = time;
        }),
    ]
}
