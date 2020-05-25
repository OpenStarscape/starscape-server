use super::{Conduit, UpdateSource};
use crate::server::Encodable;
use crate::state::{PropertyKey, State};

/// Connects a store to a server property
pub struct PropertyConduit<F> {
    store_getter: F,
}

impl<T, F> PropertyConduit<F>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
{
    pub fn new(store_getter: F) -> Self {
        Self { store_getter }
    }
}

impl<T, F> Conduit for PropertyConduit<F>
where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
{
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        Ok((*(self.store_getter)(state)?).clone().into())
    }

    fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
        Err("StoreFetcher.set_value() not implemented".into())
    }

    fn connect(&self, state: &State, property: PropertyKey) -> Result<(), String> {
        (self.store_getter)(state)?.connect(property).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }

    fn disconnect(&self, state: &State, property: PropertyKey) -> Result<(), String> {
        (self.store_getter)(state)?
            .disconnect(property)
            .map_err(|e| {
                eprintln!("Error: {}", e);
                "server_error".into()
            })
    }
}
