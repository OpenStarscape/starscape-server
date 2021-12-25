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
    /// Can only be true or false
    Flag {
        name: String,
        handler: Box<dyn Fn(&mut MasterConfig, bool) -> Result<(), Box<dyn Error>>>,
    },
    /// Has a string value
    Value {
        name: String,
        handler: Box<dyn Fn(&mut MasterConfig, &str) -> Result<(), Box<dyn Error>>>,
    },
}

impl ConfigOption {
    pub fn new_flag<F: Fn(&mut MasterConfig, bool) + 'static>(name: &str, handler: F) -> Self {
        Self::Flag {
            name: name.to_string(),
            handler: Box::new(move |conf, value| {
                handler(conf, value);
                Ok(())
            }),
        }
    }

    pub fn new_string<F: Fn(&mut MasterConfig, &str) + 'static>(name: &str, handler: F) -> Self {
        Self::Value {
            name: name.to_string(),
            handler: Box::new(move |conf, value| {
                handler(conf, value);
                Ok(())
            }),
        }
    }

    pub fn new_parsed<
        T,
        U: Fn(&str) -> Result<T, Box<dyn Error>> + 'static,
        F: Fn(&mut MasterConfig, T) + 'static,
    >(
        name: &str,
        parser: U,
        handler: F,
    ) -> Self {
        Self::Value {
            name: name.to_string(),
            handler: Box::new(move |conf, value| {
                handler(conf, parser(value)?);
                Ok(())
            }),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Flag { name, handler: _ } => name,
            Self::Value { name, handler: _ } => name,
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
            ConfigOption::Flag { name, handler } => match loaded.get_bool(&name) {
                Ok(value) => handler(&mut conf, value)?,
                Err(ConfigError::NotFound(_)) => (),
                Err(e) => return Err(e.into()),
            },
            ConfigOption::Value { name, handler } => match loaded.get_str(&name) {
                Ok(value) => handler(&mut conf, &value)?,
                Err(ConfigError::NotFound(_)) => (),
                Err(e) => return Err(e.into()),
            },
        }
    }
    Ok(conf)
}
