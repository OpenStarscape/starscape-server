use super::*;

/// Useful for creating properties that are a list of all entities with a given component.
pub struct ComponentListConduit<T: 'static> {
    /// This incantation is based on https://stackoverflow.com/a/50201389. It allows this struct to
    /// be associated with the type T while being Sync without owning a T or requiring T be Sync.
    /// Get rid of it if you can lol.
    phantom: PhantomData<dyn Fn() -> T + Send + Sync>,
}

impl<T: 'static> ComponentListConduit<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: 'static> Conduit<Value, ReadOnlyPropSetType> for ComponentListConduit<T> {
    fn output(&self, state: &State) -> RequestResult<Value> {
        let entities: Vec<Value> = state
            .components_iter::<T>()
            .map(|(entity, _)| entity.into())
            .collect();
        Ok(entities.into())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> RequestResult<()> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }
}

impl<T: 'static> Subscribable for ComponentListConduit<T> {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        state.subscribe_to_component_list::<T>(subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        state.unsubscribe_from_component_list::<T>(subscriber)
    }
}

// TODO: test
