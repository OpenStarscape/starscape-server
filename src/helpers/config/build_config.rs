extern crate config;

use super::*;
use config::{Config, ConfigError, Environment, File};

/// Get the current configuration.
pub fn build_config() -> Result<MasterConfig, Box<dyn Error>> {
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
    let mut conf = default_master();
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
