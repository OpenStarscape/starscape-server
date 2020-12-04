use super::*;

/// Connects a store to a server property
#[derive(Clone)]
pub struct ElementConduit<GetFn, SetFn> {
    getter: GetFn,
    setter: SetFn,
}

impl<T, GetFn, SetFn> ElementConduit<GetFn, SetFn>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    SetFn: Fn(&mut State, &Decoded) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    pub fn new(getter: GetFn, setter: SetFn) -> Self {
        Self { getter, setter }
    }
}

impl<T, GetFn, SetFn> Conduit for ElementConduit<GetFn, SetFn>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    SetFn: Fn(&mut State, &Decoded) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        Ok((*(self.getter)(state)?).clone().into())
    }

    fn set_value(&self, state: &mut State, value: &Decoded) -> Result<(), String> {
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
