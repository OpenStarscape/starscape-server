use super::*;

pub type ConfigEntrySetterFn<'a, T> = Box<dyn FnMut(T, String) -> Result<(), Box<dyn Error>> + 'a>;

pub enum ConfigEntrySetter<'a> {
    Bool(ConfigEntrySetterFn<'a, bool>),
    String(ConfigEntrySetterFn<'a, String>),
    #[allow(dead_code)]
    Int(ConfigEntrySetterFn<'a, i64>),
    Float(ConfigEntrySetterFn<'a, f64>),
}

impl<'a> std::fmt::Display for ConfigEntrySetter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bool(_) => "bool",
                Self::String(_) => "string",
                Self::Int(_) => "int",
                Self::Float(_) => "float",
            }
        )
    }
}

pub trait ConfigEntry {
    fn name(&self) -> &str;
    fn setter(&mut self) -> ConfigEntrySetter;
    fn apply_to(&self, target: &mut MasterConfig) -> Result<(), Box<dyn Error>>;
}

impl dyn ConfigEntry {
    pub fn new_bool<F: Fn(&mut MasterConfig, bool) + 'static>(
        name: &str,
        default_value: bool,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, default_value, apply, |target| {
            ConfigEntrySetter::Bool(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_string<F: Fn(&mut MasterConfig, String) + 'static>(
        name: &str,
        default_value: &str,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, default_value.to_string(), apply, |target| {
            ConfigEntrySetter::String(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    #[allow(dead_code)]
    pub fn new_int<F: Fn(&mut MasterConfig, i64) + 'static>(
        name: &str,
        default_value: i64,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, default_value, apply, |target| {
            ConfigEntrySetter::Int(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_float<F: Fn(&mut MasterConfig, f64) + 'static>(
        name: &str,
        default_value: f64,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, default_value, apply, |target| {
            ConfigEntrySetter::Float(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }
}

struct SetterTarget<T> {
    pub value: T,
    /// Some if the value is not default, describes how it was set
    pub source: Option<String>,
}

struct ConfigEntryImpl<T> {
    name: String,
    target: SetterTarget<T>,
    apply_fn: Box<dyn Fn(&mut MasterConfig, T) -> Result<(), Box<dyn Error>>>,
    setter_builder: Box<dyn Fn(&mut SetterTarget<T>) -> ConfigEntrySetter>,
}

impl<T> ConfigEntryImpl<T> {
    pub fn new<F, B>(name: &str, default_value: T, apply: F, setter_builder: B) -> Box<Self>
    where
        F: Fn(&mut MasterConfig, T) + 'static,
        B: Fn(&mut SetterTarget<T>) -> ConfigEntrySetter + 'static,
    {
        Box::new(ConfigEntryImpl {
            name: name.to_string(),
            target: SetterTarget {
                value: default_value,
                source: None,
            },
            apply_fn: Box::new(move |conf, value| {
                apply(conf, value);
                Ok(())
            }),
            setter_builder: Box::new(setter_builder),
        })
    }
}

impl<T: Clone> ConfigEntry for ConfigEntryImpl<T> {
    fn name(&self) -> &str {
        &self.name
    }

    fn setter(&mut self) -> ConfigEntrySetter {
        (self.setter_builder)(&mut self.target)
    }

    fn apply_to(&self, target: &mut MasterConfig) -> Result<(), Box<dyn Error>> {
        (self.apply_fn)(target, self.target.value.clone())
    }
}

pub struct ConfigBuilder {
    entries: Vec<Box<dyn ConfigEntry>>,
}

impl ConfigBuilder {
    pub fn new(entries: Vec<Box<dyn ConfigEntry>>) -> Self {
        Self { entries }
    }

    pub fn entry(&mut self, name: &str) -> Option<ConfigEntrySetter> {
        // Not the most efficient but good enough for the usecase
        for entry in &mut self.entries {
            if entry.name() == name {
                return Some(entry.setter());
            }
        }
        None
    }

    pub fn apply_to(&self, target: &mut MasterConfig) -> Result<(), Box<dyn Error>> {
        for entry in &self.entries {
            entry
                .apply_to(target)
                .map_err(|e| format!("{} configuration option: {}", entry.name(), e))?;
        }
        Ok(())
    }
}
