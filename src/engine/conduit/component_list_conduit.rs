use super::*;

/// Useful for creating properties that are a list of all entities with a given component
pub struct ComponentListConduit<T: 'static>(PhantomData<T>);

impl<T: 'static> ComponentListConduit<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Conduit<Encodable, ReadOnlyPropSetType> for ComponentListConduit<T> {
    fn output(&self, state: &State) -> Result<Encodable, String> {
        let entities: Vec<Encodable> = state
            .components_iter::<T>()
            .map(|(entity, _)| entity.into())
            .collect();
        Ok(entities.into())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> Result<(), String> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        state
            .subscribe_to_component_list::<T>(subscriber)
            .map_err(|e| {
                error!("subscribing to all {} components: {}", type_name::<T>(), e);
                "server_error".into()
            })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        state
            .unsubscribe_from_component_list::<T>(subscriber)
            .map_err(|e| {
                error!(
                    "unsubscribing from all {} components: {}",
                    type_name::<T>(),
                    e
                );
                "server_error".into()
            })
    }
}

// TODO: test
