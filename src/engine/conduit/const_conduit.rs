use super::*;

/// Produces a value that never changes
pub struct ConstConduit<T> {
    value: T,
}

impl<T> ConstConduit<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Conduit<T, ReadOnlyPropSetType> for ConstConduit<T>
where
    T: Clone + Send + Sync,
{
    fn output(&self, state: &State) -> RequestResult<T> {
        Ok(self.value.clone())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> RequestResult<()> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }
}
impl<T> Subscribable for ConstConduit<T> {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        Ok(())
    }
    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        Ok(())
    }
}
