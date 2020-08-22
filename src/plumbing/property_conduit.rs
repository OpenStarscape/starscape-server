use std::sync::{Arc, Weak};

use super::*;
use crate::{server::Encodable, state::State};

/// Connects a store to a server property
#[derive(Clone)]
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
    F: Clone + 'static,
{
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        Ok((*(self.store_getter)(state)?).clone().into())
    }

    fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
        Err("StoreFetcher.set_value() not implemented".into())
    }

    fn subscribe(
        &self,
        state: &State,
        subscriber: &Arc<dyn NotificationSink>,
    ) -> Result<(), String> {
        (self.store_getter)(state)?
            .subscribe(subscriber)
            .map_err(|e| {
                eprintln!("Error: {}", e);
                "server_error".into()
            })
    }

    fn unsubscribe(
        &self,
        state: &State,
        subscriber: &Weak<dyn NotificationSink>,
    ) -> Result<(), String> {
        (self.store_getter)(state)?
            .unsubscribe(subscriber)
            .map_err(|e| {
                eprintln!("Error: {}", e);
                "server_error".into()
            })
    }
}
