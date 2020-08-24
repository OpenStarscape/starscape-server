use super::*;

/// Connects a store to a server property
#[derive(Clone)]
pub struct PropertyConduit<GetFn, SetFn> {
    getter: GetFn,
    setter: SetFn,
}

impl<T, GetFn, SetFn> PropertyConduit<GetFn, SetFn>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
    SetFn: Fn(&mut State, &Decodable) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    pub fn new(getter: GetFn, setter: SetFn) -> Self {
        Self { getter, setter }
    }
}

impl<T, GetFn, SetFn> Conduit for PropertyConduit<GetFn, SetFn>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
    SetFn: Fn(&mut State, &Decodable) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        Ok((*(self.getter)(state)?).clone().into())
    }

    fn set_value(&self, state: &mut State, value: &Decodable) -> Result<(), String> {
        (self.setter)(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        (self.getter)(state)?.subscribe(subscriber).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        (self.getter)(state)?.unsubscribe(subscriber).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }
}
