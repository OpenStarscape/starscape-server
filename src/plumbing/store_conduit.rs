use super::{Conduit, Store};
use crate::connection::Value;
use crate::state::{PropertyKey, State};

pub struct StoreConduit<F> {
    store_getter: F,
}

impl<T, F> StoreConduit<F>
where
    T: Into<Value> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a Store<T>, String>,
{
    pub fn new(store_getter: F) -> Self {
        Self { store_getter }
    }
}

impl<T, F> Conduit for StoreConduit<F>
where
    T: Into<Value> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a Store<T>, String>,
{
    fn get_value(&self, state: &State) -> Result<Value, String> {
        Ok((*(self.store_getter)(state)?.value()).clone().into())
    }

    fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
        Err("StoreFetcher.set_value() not implemented".into())
    }

    fn connect(&self, state: &State, conduit: PropertyKey) -> Result<(), String> {
        (self.store_getter)(state)?.connect(conduit);
        Ok(())
    }

    fn disconnect(&self, state: &State, conduit: PropertyKey) -> Result<(), String> {
        (self.store_getter)(state)?.disconnect(conduit);
        Ok(())
    }
}