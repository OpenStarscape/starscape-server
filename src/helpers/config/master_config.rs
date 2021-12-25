use super::*;

/// Configuration for the whole starscape-server program
pub struct MasterConfig {
    pub max_game_time: f64,
    pub server: ServerConfig,
}

pub fn default_master() -> MasterConfig {
    MasterConfig {
        max_game_time: 1200.0,
        server: ServerConfig {
            tcp: Some(SocketAddrConfig::new_loopback()),
            http: Some(HttpServerConfig {
                static_content_path: None,
                enable_websockets: true,
                enable_webrtc_experimental: false,
                server_type: HttpServerType::Unencrypted(SocketAddrConfig::new_loopback()),
            }),
        },
    }
}
