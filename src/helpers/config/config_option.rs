use super::*;

/// Configuration option with function that will handle it. Name is lower case with underscores.
pub enum ConfigOption {
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

pub type ConfigOptionSchema = Vec<ConfigOption>;

pub enum ConfigOptionValue {
    Bool(bool),
    String(String),
}
