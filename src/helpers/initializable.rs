use super::*;

/// a struct that can be initialized after creation, and reinitialized with the same value.
pub struct Initializable<T> {
    value: Option<T>,
}

impl<T: PartialEq + Clone> Initializable<T> {
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Initializes with a clone of the given value. If already initialized with equal value does
    /// nothing and returns success. If alreaded initialized with differnt value does nothing and
    /// returns error.
    pub fn try_init_with_clone(&mut self, value: &T) -> Result<(), Box<dyn Error>> {
        if let Some(prev) = &self.value {
            if prev != value {
                Err(format!(
                    "tried to reinitialize {} with differnt value",
                    type_name::<Self>()
                )
                .into())
            } else {
                Ok(())
            }
        } else {
            self.value = Some(value.clone());
            Ok(())
        }
    }

    /// Returns the value, or an error if it hans't been initialized
    pub fn get(&self) -> Result<&T, Box<dyn Error>> {
        self.value
            .as_ref()
            .ok_or_else(|| format!("{} was not initialized", type_name::<Self>()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_initialize() {
        let mut i = Initializable::new();
        i.try_init_with_clone(&7).unwrap();
        assert_eq!(i.get().unwrap(), &7);
    }

    #[test]
    fn get_fails_when_uninitialized() {
        let i = Initializable::<i32>::new();
        assert!(i.get().is_err());
    }

    #[test]
    fn can_initialize_multiple_times_with_same() {
        let mut i = Initializable::new();
        i.try_init_with_clone(&7).unwrap();
        i.try_init_with_clone(&7).unwrap();
        i.get().unwrap();
        i.try_init_with_clone(&7).unwrap();
    }

    #[test]
    fn errors_when_initialized_with_different() {
        let mut i = Initializable::new();
        i.try_init_with_clone(&7).unwrap();
        assert!(i.try_init_with_clone(&3).is_err());
    }

    #[test]
    fn is_not_updated_after_err() {
        let mut i = Initializable::new();
        i.try_init_with_clone(&7).unwrap();
        let _ = i.try_init_with_clone(&3).is_err();
        assert_eq!(i.get().unwrap(), &7);
    }
}
