extern crate config;

use super::*;
use config::{Config, ConfigError, Environment, File};

/// Configuration for the whole starscape-server program
pub struct MasterConfig {
    pub max_game_time: f64,
    pub server: ServerConfig,
}

/// Configuration option with function that will handle it. Name is lower case with underscores.
enum ConfigOption {
    Bool {
        name: String,
        handler: Box<dyn Fn(&mut MasterConfig, bool) -> Result<(), Box<dyn Error>>>,
    },
    String {
        name: String,
        handler: Box<dyn Fn(&mut MasterConfig, &str) -> Result<(), Box<dyn Error>>>,
    },
}

impl ConfigOption {
    pub fn new_bool<F: Fn(&mut MasterConfig, bool) -> Result<(), Box<dyn Error>> + 'static>(
        name: &str,
        handler: F,
    ) -> Self {
        Self::Bool {
            name: name.to_string(),
            handler: Box::new(handler),
        }
    }
    pub fn new_string<F: Fn(&mut MasterConfig, &str) -> Result<(), Box<dyn Error>> + 'static>(
        name: &str,
        handler: F,
    ) -> Self {
        Self::String {
            name: name.to_string(),
            handler: Box::new(handler),
        }
    }
}

fn default_config() -> MasterConfig {
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

/// These options will be called in order of returned vec (NOT in the order the user specifies the options) if the user
/// specifies them. An option's handler will not be called if is not specified by the user.
fn option_list() -> Vec<ConfigOption> {
    vec![
        ConfigOption::new_bool("tcp", |conf, enable| {
            conf.server.tcp = if enable {
                Some(SocketAddrConfig::new_loopback())
            } else {
                None
            };
            Ok(())
        }),
        ConfigOption::new_bool("websockets", |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_websockets = enable;
            };
            Ok(())
        }),
        ConfigOption::new_bool("webrtc", |conf, enable| {
            if let Some(http) = &mut conf.server.http {
                http.enable_webrtc_experimental = enable;
            };
            Ok(())
        }),
        ConfigOption::new_bool("https", |conf, enable| {
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
            Ok(())
        }),
        ConfigOption::new_string("http_content", |conf, path| {
            if let Some(http) = &mut conf.server.http {
                http.static_content_path = Some(path.to_string());
            };
            Ok(())
        }),
        ConfigOption::new_string("max_game_time", |conf, time| {
            conf.max_game_time = time.parse::<f64>()?;
            Ok(())
        }),
    ]
}

/// Get the current configuration.
pub fn get() -> Result<MasterConfig, Box<dyn Error>> {
    // TODO: use toml crate and our own env parsing code instead of config crate
    // TODO: use command line arguments
    // TODO: do not allow unknown config options
    // TODO: accumulate multiple config errors
    // TODO: IPv6 config???
    // TODO: verify the final config is valid (paths exist, etc)
    let mut loaded = Config::default();
    loaded
        .merge(File::with_name("starscape"))?
        .merge(Environment::with_prefix("STARSCAPE"))
        .unwrap();
    let mut conf = default_config();
    for option in option_list() {
        match option {
            ConfigOption::Bool { name, handler } => match loaded.get_bool(&name) {
                Ok(value) => handler(&mut conf, value)?,
                Err(ConfigError::NotFound(_)) => (),
                Err(e) => return Err(e.into()),
            },
            ConfigOption::String { name, handler } => match loaded.get_str(&name) {
                Ok(value) => handler(&mut conf, &value)?,
                Err(ConfigError::NotFound(_)) => (),
                Err(e) => return Err(e.into()),
            },
        }
    }
    Ok(conf)
}
