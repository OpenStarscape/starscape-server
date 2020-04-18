/// This module contains everything concerned with shuffling data
/// between the state and connections
mod conduit;
mod conduit_property;
mod property;
mod store;
mod store_conduit;

use conduit::Conduit;
use conduit_property::ConduitProperty;
use store_conduit::StoreConduit;

pub use property::Property;
pub use store::Store;

use crate::connection::Value;
use crate::state::{EntityKey, PropertyKey, State};

pub fn new_property<T, F: 'static>(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    store_getter: F,
) where
    T: Into<Value> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a Store<T>, String>,
{
    let conduit = state.properties.insert_with_key(|key| {
        Box::new(ConduitProperty::new(
            entity,
            name,
            Box::new(StoreConduit::new(key, store_getter)),
        ))
    });
    state.entities[entity].add_property(name, conduit);
}

// TODO: test that new_property() adds property to entity
