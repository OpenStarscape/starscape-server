use super::*;

pub fn build_config_with(
    args: Vec<String>,
    fs: Filesystem,
) -> Result<MasterConfig, Box<dyn Error>> {
    // TODO: use environment variables
    // TODO: use command line arguments
    // TODO: do not allow unknown config options
    // TODO: accumulate multiple config errors
    // TODO: IPv6 config???
    // TODO: verify the final config is valid (paths exist, etc)
    let mut entries = Vec::new();
    entries.append(&mut config_entries());
    entries.append(&mut server_config_entries());
    let mut builder = ConfigBuilder::new(entries);
    if fs.is_file(DEFAULT_TOML_PATH) {
        load_toml(DEFAULT_TOML_PATH, &mut builder, fs)?;
    }
    parse_args(&mut builder, args)?;
    let mut conf = MasterConfig::default();
    builder.apply_to(&mut conf)?;
    Ok(conf)
}

/// Build a configuration
pub fn build_config() -> Result<MasterConfig, Box<dyn Error>> {
    build_config_with(std::env::args().collect(), real_filesystem())
}
