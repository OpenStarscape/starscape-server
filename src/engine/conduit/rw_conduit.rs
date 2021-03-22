use super::*;

/// Connects an element to the conduit system
pub struct RWConduit<OFn, IFn> {
    output_fn: OFn,
    input_fn: IFn,
}

impl<T, OFn, IFn> RWConduit<OFn, IFn>
where
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    IFn: Fn(&mut State, T) -> RequestResult<()>,
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
}

impl<T, OFn, IFn> Conduit<T, T> for RWConduit<OFn, IFn>
where
    T: Clone,
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    IFn: Fn(&mut State, T) -> RequestResult<()>,
    OFn: Send + Sync + 'static,
    IFn: Send + Sync + 'static,
{
    fn output(&self, state: &State) -> RequestResult<T> {
        Ok((*(self.output_fn)(state)?).clone())
    }

    fn input(&self, state: &mut State, value: T) -> RequestResult<()> {
        (self.input_fn)(state, value)
    }
}

impl<T, OFn, IFn> Subscribable for RWConduit<OFn, IFn>
where
    T: Clone,
    for<'a> OFn: Fn(&'a State) -> RequestResult<&'a Element<T>>,
    IFn: Fn(&mut State, T) -> RequestResult<()>,
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
