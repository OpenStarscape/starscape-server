use super::*;

pub fn build_config_with(fs: Filesystem) -> Result<MasterConfig, Box<dyn Error>> {
    // TODO: use environment variables
    // TODO: use command line arguments
    // TODO: do not allow unknown config options
    // TODO: accumulate multiple config errors
    // TODO: IPv6 config???
    // TODO: verify the final config is valid (paths exist, etc)
    let mut builder = ConfigBuilder::new(config_entries());
    if fs.is_file(DEFAULT_TOML_PATH) {
        load_toml(DEFAULT_TOML_PATH, &mut builder, fs)?;
    }
    let mut conf = MasterConfig::default();
    builder.apply_to(&mut conf)?;
    Ok(conf)
}

/// Build a configuration
pub fn build_config() -> Result<MasterConfig, Box<dyn Error>> {
    build_config_with(real_filesystem())
}
