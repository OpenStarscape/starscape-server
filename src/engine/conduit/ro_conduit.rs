use super::*;

/// Connects a read-only element to the conduit system
pub struct ROConduit<GetFn> {
    getter: GetFn,
}

impl<T, GetFn> ROConduit<GetFn>
where
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    GetFn: 'static,
{
    #[must_use]
    pub fn new(getter: GetFn) -> Self {
        Self { getter }
    }
}

impl<T, GetFn> Conduit<T, ReadOnlyPropSetType> for ROConduit<GetFn>
where
    T: Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    GetFn: 'static,
{
    fn get_value(&self, state: &State) -> Result<T, String> {
        Ok((*(self.getter)(state)?).clone())
    }

    fn set_value(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> Result<(), String> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        (self.getter)(state)?.subscribe(subscriber).map_err(|e| {
            error!("subscribing to Element<{}>: {}", type_name::<T>(), e);
            "server_error".into()
        })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        (self.getter)(state)?.unsubscribe(subscriber).map_err(|e| {
            error!("unsubscribing from Element<{}>: {}", type_name::<T>(), e);
            "server_error".into()
        })
    }
}
