extern crate toml;

use super::*;

pub const DEFAULT_TOML_PATH: &str = "starscape.toml";

fn value_to_config_value(value: toml::Value) -> Result<ConfigOptionValue, Box<dyn Error>> {
    match value {
        toml::Value::String(s) => Ok(ConfigOptionValue::String(s)),
        toml::Value::Integer(i) => Ok(ConfigOptionValue::String(i.to_string())),
        toml::Value::Float(f) => Ok(ConfigOptionValue::String(f.to_string())),
        toml::Value::Boolean(b) => Ok(ConfigOptionValue::Bool(b)),
        v @ _ => Err(format!("{:?} is not a valid config option", v).into()),
    }
}

pub fn load_toml(path: &str) -> Result<HashMap<String, ConfigOptionValue>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    let parsed = contents.parse::<toml::Value>()?;
    match parsed {
        toml::Value::Table(table) => {
            let mut result = HashMap::new();
            for (name, value) in table {
                result.insert(name, value_to_config_value(value)?);
            }
            Ok(result)
        }
        _ => Err(format!("toplevel value of {} is not a table", path).into()),
    }
}
