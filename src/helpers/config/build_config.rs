use super::*;

/// Get the current configuration.
pub fn build_config() -> Result<MasterConfig, Box<dyn Error>> {
    // TODO: use environment variables
    // TODO: use command line arguments
    // TODO: do not allow unknown config options
    // TODO: accumulate multiple config errors
    // TODO: IPv6 config???
    // TODO: verify the final config is valid (paths exist, etc)
    let mut values = HashMap::new();
    if std::path::Path::new(DEFAULT_TOML_PATH).is_file() {
        values.extend(load_toml(DEFAULT_TOML_PATH)?);
    }
    let mut conf = default_master();
    for option in option_list() {
        if let Some(value) = values.get(option.name()) {
            match option {
                ConfigOption::Flag { name, handler } => {
                    if let ConfigOptionValue::Bool(v) = value {
                        handler(&mut conf, *v)?;
                    } else {
                        return Err(format!("{} has invalid type", name).into());
                    }
                }
                ConfigOption::Value { name, handler } => {
                    if let ConfigOptionValue::String(v) = value {
                        handler(&mut conf, v)?;
                    } else {
                        return Err(format!("{} has invalid type", name).into());
                    }
                }
            }
        }
    }
    Ok(conf)
}
