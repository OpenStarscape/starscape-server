use super::*;

/// Useful for creating properties that are a list of all entities with a given component
pub struct ComponentListConduit<T: 'static>(PhantomData<T>);

// Required because of https://github.com/rust-lang/rust/issues/26925
impl<T> Clone for ComponentListConduit<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> ComponentListConduit<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Conduit for ComponentListConduit<T> {
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        let entities: Vec<EntityKey> = state
            .components_iter::<T>()
            .map(|(entity, _)| entity)
            .collect();
        Ok(entities.into())
    }

    fn set_value(&self, _state: &mut State, _value: &Decoded) -> Result<(), String> {
        Err("read_only_property".into())
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
