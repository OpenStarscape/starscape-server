use super::*;

/// Useful for creating properties that are a list of all entities with a given component
pub struct ComponentListConduit<T: 'static>(PhantomData<T>);

impl<T: 'static> ComponentListConduit<T> {
    pub fn new() -> Self {
        Self(PhantomData)
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

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        state
            .subscribe_to_component_list::<T>(subscriber)
            .map_err(|e| {
                InternalError(format!(
                    "failed to subscribe to {} component list: {}",
                    type_name::<T>(),
                    e
                ))
            })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        state
            .unsubscribe_from_component_list::<T>(subscriber)
            .map_err(|e| {
                InternalError(format!(
                    "failed to unsubscribe from {} component list: {}",
                    type_name::<T>(),
                    e
                ))
            })
    }
}

// TODO: test
