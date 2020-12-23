use super::*;

/// Connects a read-only element to the conduit system
pub struct ROConduit<OFn> {
    output_fn: OFn,
}

impl<T, OFn> ROConduit<OFn>
where
    for<'a> OFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    OFn: 'static,
{
    #[must_use]
    pub fn new(output_fn: OFn) -> Self {
        Self { output_fn }
    }
}

impl<T, OFn> Conduit<T, ReadOnlyPropSetType> for ROConduit<OFn>
where
    T: Clone,
    for<'a> OFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    OFn: 'static,
{
    fn output(&self, state: &State) -> Result<T, String> {
        Ok((*(self.output_fn)(state)?).clone())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> Result<(), String> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        (self.output_fn)(state)?.subscribe(subscriber).map_err(|e| {
            error!("subscribing to Element<{}>: {}", type_name::<T>(), e);
            "server_error".into()
        })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        (self.output_fn)(state)?
            .unsubscribe(subscriber)
            .map_err(|e| {
                error!("unsubscribing from Element<{}>: {}", type_name::<T>(), e);
                "server_error".into()
            })
    }
}
