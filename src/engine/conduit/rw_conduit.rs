use super::*;

/// Connects an element to the conduit system
pub struct RWConduit<GetFn, SetFn> {
    getter: GetFn,
    setter: SetFn,
}

impl<T, GetFn, SetFn> RWConduit<GetFn, SetFn>
where
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    SetFn: Fn(&mut State, T) -> Result<(), String>,
    GetFn: 'static,
    SetFn: 'static,
{
    #[must_use]
    pub fn new(getter: GetFn, setter: SetFn) -> Self {
        Self { getter, setter }
    }
}

impl<T, GetFn, SetFn> Conduit<T, T> for RWConduit<GetFn, SetFn>
where
    T: Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    SetFn: Fn(&mut State, T) -> Result<(), String>,
    GetFn: 'static,
    SetFn: 'static,
{
    fn get_value(&self, state: &State) -> Result<T, String> {
        Ok((*(self.getter)(state)?).clone())
    }

    fn set_value(&self, state: &mut State, value: T) -> Result<(), String> {
        (self.setter)(state, value)
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
