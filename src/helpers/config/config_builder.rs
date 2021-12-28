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
    fn help(&self) -> &str;
    fn setter(&mut self) -> ConfigEntrySetter;
    fn apply_to(&self, target: &mut MasterConfig) -> Result<(), Box<dyn Error>>;
}

impl dyn ConfigEntry {
    pub fn new_bool<F>(name: &str, help: &str, default_value: bool, apply: F) -> Box<Self>
    where
        F: Fn(&mut MasterConfig, bool, Option<&str>) -> Result<(), Box<dyn Error>> + 'static,
    {
        ConfigEntryImpl::new(name, help, default_value, apply, |target| {
            ConfigEntrySetter::Bool(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_string<
        F: Fn(&mut MasterConfig, String, Option<&str>) -> Result<(), Box<dyn Error>> + 'static,
    >(
        name: &str,
        help: &str,
        default_value: &str,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, help, default_value.to_string(), apply, |target| {
            ConfigEntrySetter::String(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_int<F>(name: &str, help: &str, default_value: i64, apply: F) -> Box<Self>
    where
        F: Fn(&mut MasterConfig, i64, Option<&str>) -> Result<(), Box<dyn Error>> + 'static,
    {
        ConfigEntryImpl::new(name, help, default_value, apply, |target| {
            ConfigEntrySetter::Int(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_float<
        F: Fn(&mut MasterConfig, f64, Option<&str>) -> Result<(), Box<dyn Error>> + 'static,
    >(
        name: &str,
        help: &str,
        default_value: f64,
        apply: F,
    ) -> Box<Self> {
        ConfigEntryImpl::new(name, help, default_value, apply, |target| {
            ConfigEntrySetter::Float(Box::new(move |value, source| {
                target.value = value;
                target.source = Some(source);
                Ok(())
            }))
        })
    }

    pub fn new_enum(name: &str, help: &str, variants: Vec<ConfigEntryVariant>) -> Box<Self> {
        assert!(variants.len() > 0);
        let mut help = help.to_string();
        for variant in &variants {
            help.push_str(&format!("\n  {}: {}", variant.name, variant.help))
        }
        ConfigEntryImpl::new(
            &name,
            &help,
            variants[0].name.clone(),
            move |conf, value, source| {
                for variant in &variants {
                    if variant.name == value {
                        (variant.apply_fn)(conf, source);
                        return Ok(());
                    }
                }
                Err(format!(
                    "{} has invalid value {}, valid options are {}",
                    source.expect("default config enum value invalid (should not be possible)"),
                    value,
                    variants
                        .iter()
                        .map(|v| v.name.clone())
                        .collect::<Vec<String>>()
                        .join(", "),
                )
                .into())
            },
            |target| {
                ConfigEntrySetter::String(Box::new(move |value, source| {
                    target.value = value;
                    target.source = Some(source);
                    Ok(())
                }))
            },
        )
    }

    pub fn new_enum_variant<F: Fn(&mut MasterConfig, Option<&str>) + 'static>(
        name: &str,
        help: &str,
        apply: F,
    ) -> ConfigEntryVariant {
        ConfigEntryVariant {
            name: name.to_string(),
            help: help.to_string(),
            apply_fn: Box::new(apply),
        }
    }
}

pub struct ConfigEntryVariant {
    pub name: String,
    pub help: String,
    pub apply_fn: Box<dyn Fn(&mut MasterConfig, Option<&str>)>,
}

struct SetterTarget<T> {
    pub value: T,
    /// Some if the value is not default, describes how it was set
    pub source: Option<String>,
}

struct ConfigEntryImpl<T> {
    name: String,
    help: String,
    target: SetterTarget<T>,
    apply_fn: Box<dyn Fn(&mut MasterConfig, T, Option<&str>) -> Result<(), Box<dyn Error>>>,
    setter_builder: Box<dyn Fn(&mut SetterTarget<T>) -> ConfigEntrySetter>,
}

impl<T> ConfigEntryImpl<T> {
    pub fn new<APPLY, SETTER>(
        name: &str,
        help: &str,
        default_value: T,
        apply_fn: APPLY,
        setter_builder: SETTER,
    ) -> Box<Self>
    where
        APPLY: Fn(&mut MasterConfig, T, Option<&str>) -> Result<(), Box<dyn Error>> + 'static,
        SETTER: Fn(&mut SetterTarget<T>) -> ConfigEntrySetter + 'static,
    {
        Box::new(ConfigEntryImpl {
            name: name.to_string(),
            help: help.to_string(),
            target: SetterTarget {
                value: default_value,
                source: None,
            },
            apply_fn: Box::new(apply_fn),
            setter_builder: Box::new(setter_builder),
        })
    }
}

impl<T: Clone + 'static> ConfigEntry for ConfigEntryImpl<T> {
    fn name(&self) -> &str {
        &self.name
    }

    fn help(&self) -> &str {
        &self.help
    }

    fn setter(&mut self) -> ConfigEntrySetter {
        (self.setter_builder)(&mut self.target)
    }

    fn apply_to(&self, target: &mut MasterConfig) -> Result<(), Box<dyn Error>> {
        (self.apply_fn)(
            target,
            self.target.value.clone(),
            self.target.source.as_deref(),
        )
    }
}

pub struct ConfigBuilder {
    entries: Vec<Box<dyn ConfigEntry>>,
}

impl ConfigBuilder {
    pub fn new(entries: Vec<Box<dyn ConfigEntry>>) -> Self {
        let mut names = HashSet::new();
        for entry in &entries {
            if !names.insert(entry.name()) {
                panic!("duplicate configuration entry {}", entry.name());
            }
        }
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
