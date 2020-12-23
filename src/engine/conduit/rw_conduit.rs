use super::*;

/// Connects an element to the conduit system
pub struct RWConduit<OFn, IFn> {
    output_fn: OFn,
    input_fn: IFn,
}

impl<T, OFn, IFn> RWConduit<OFn, IFn>
where
    for<'a> OFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    IFn: Fn(&mut State, T) -> Result<(), String>,
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
    for<'a> OFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    IFn: Fn(&mut State, T) -> Result<(), String>,
    OFn: 'static,
    IFn: 'static,
{
    fn output(&self, state: &State) -> Result<T, String> {
        Ok((*(self.output_fn)(state)?).clone())
    }

    fn input(&self, state: &mut State, value: T) -> Result<(), String> {
        (self.input_fn)(state, value)
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
