use super::*;

/// Connects a read-only element to the conduit system
pub struct ROConduit<OFn> {
    output_fn: OFn,
}

impl<T, OFn> ROConduit<OFn>
where
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
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
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    OFn: Send + Sync + 'static,
{
    fn output(&self, state: &State) -> RequestResult<T> {
        Ok((*(self.output_fn)(state)?).clone())
    }

    fn input(&self, _state: &mut State, _value: ReadOnlyPropSetType) -> RequestResult<()> {
        // ReadOnlyPropSetType can't be instantiated, so this can't be called
        std::unreachable!()
    }
}
impl<T, OFn> Subscribable for ROConduit<OFn>
where
    T: Clone,
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    OFn: Send + Sync + 'static,
{
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        (self.output_fn)(state)?
            .subscribe(subscriber, &state.notif_queue)
            .map_err(|e| {
                InternalError(format!(
                    "failed to subscribe to to Element<{}>: {}",
                    type_name::<T>(),
                    e
                ))
            })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        (self.output_fn)(state)?
            .unsubscribe(subscriber)
            .map_err(|e| {
                InternalError(format!(
                    "failed to unsubscribe from Element<{}>: {}",
                    type_name::<T>(),
                    e
                ))
            })
    }
}
