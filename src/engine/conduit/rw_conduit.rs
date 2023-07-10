use super::*;

/// Connects an element to the conduit system
pub struct RWConduit<OFn, IFn> {
    output_fn: OFn,
    input_fn: IFn,
}

impl<T, OFn, IFn> RWConduit<OFn, IFn>
where
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    for<'a> IFn: Fn(&'a mut State) -> RequestResult<&'a mut Element<T>>,
    OFn: 'static,
    IFn: 'static,
{
    #[must_use]
    pub fn new(output_fn: OFn, input_fn: IFn) -> Self {
        Self {
            output_fn,
            input_fn,
        }
    }

    #[must_use]
    pub fn new_into(output_fn: OFn, input_fn: IFn) -> TryIntoConduit<Self, T, T> {
        TryIntoConduit::new(Self::new(output_fn, input_fn))
    }
}

impl<T, OFn, IFn> Conduit<T, T> for RWConduit<OFn, IFn>
where
    T: Clone + PartialEq,
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    for<'a> IFn: Fn(&'a mut State) -> RequestResult<&'a mut Element<T>>,
    OFn: Send + Sync + 'static,
    IFn: Send + Sync + 'static,
{
    fn output(&self, state: &State) -> RequestResult<T> {
        Ok((*(self.output_fn)(state)?).clone())
    }

    fn input(&self, state: &mut State, value: T) -> RequestResult<()> {
        Ok((self.input_fn)(state)?.set(value))
    }
}

impl<T, OFn, IFn> Subscribable for RWConduit<OFn, IFn>
where
    T: Clone,
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    for<'a> IFn: Fn(&'a mut State) -> RequestResult<&'a mut Element<T>>,
    OFn: Send + Sync + 'static,
    IFn: Send + Sync + 'static,
{
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        (self.output_fn)(state)?.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        (self.output_fn)(state)?.unsubscribe(state, subscriber)
    }
}
