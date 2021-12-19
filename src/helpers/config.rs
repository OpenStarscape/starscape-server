extern crate config;

use super::*;
use config::{Config, Environment, File};

/// Configuration for the whole starscape-server program
pub struct MasterConfig {
    pub max_game_time: f64,
    pub server: ServerConfig,
}

/// Get the current configuration.
pub fn get() -> Result<MasterConfig, Box<dyn Error>> {
    let mut conf = Config::default();
    conf.set_default("tcp", true).unwrap();
    conf.set_default("websockets", true).unwrap();
    conf.set_default("webrtc", true).unwrap();
    conf.set_default("https", true).unwrap();
    conf.set_default("http_content", "").unwrap();
    conf.set_default("max_game_time", 1200.0).unwrap();
    conf.merge(File::with_name("starscape"))?
        .merge(Environment::with_prefix("STARSCAPE"))
        .unwrap();
    let tcp = conf.get_bool("tcp")?;
    let websockets = conf.get_bool("websockets")?;
    let webrtc = conf.get_bool("webrtc")?;
    let https = conf.get_bool("https")?;
    let http_content = conf.get_str("http_content")?;
    let max_game_time = conf.get_float("max_game_time")?;
    Ok(MasterConfig {
        max_game_time,
        server: ServerConfig {
            tcp: if tcp {
                Some(SocketAddrConfig::new(false))
            } else {
                None
            },
            http: Some(HttpServerConfig {
                static_content_path: if http_content == "" {
                    None
                } else {
                    Some(http_content)
                },
                enable_websockets: websockets,
                webrtc_experimental: if webrtc {
                    Some(SocketAddrConfig::new(false))
                } else {
                    None
                },
                server_type: if https {
                    HttpServerType::Encrypted {
                        socket_addr: SocketAddrConfig::new(false),
                        cert_path: "../ssl/cert.pem".to_string(),
                        key_path: "../ssl/privkey.pem".to_string(),
                        enable_http_to_https_redirect: true,
                    }
                } else {
                    // TODO: stop conflating unencrypted with loopback
                    let mut addr = SocketAddrConfig::new(true);
                    addr.port = Some(56_560);
                    HttpServerType::Unencrypted(addr)
                },
            }),
        },
    })
}
