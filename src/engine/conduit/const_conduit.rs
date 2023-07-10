use super::*;

/// Produces a value that never changes
pub struct ConstConduit<T> {
    value: T,
}

impl<T> ConstConduit<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn new_into(value: T) -> TryIntoConduit<Self, T, ReadOnlyPropSetType> {
        TryIntoConduit::new(Self::new(value))
    }
}

impl<T> Conduit<T, ReadOnlyPropSetType> for ConstConduit<T>
where
    T: Clone + Send + Sync,
{
    fn output(&self, _: &State) -> RequestResult<T> {
        Ok(self.value.clone())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> RequestResult<()> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }
}
impl<T> Subscribable for ConstConduit<T> {
    fn subscribe(&self, _: &State, _: &Arc<dyn Subscriber>) -> RequestResult<()> {
        Ok(())
    }
    fn unsubscribe(&self, _: &State, _: &Weak<dyn Subscriber>) -> RequestResult<()> {
        Ok(())
    }
}
