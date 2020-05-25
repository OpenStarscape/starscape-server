/// This module contains everything concerned with shuffling data
/// between the state and connections
mod conduit;
mod conduit_property;
mod notification_source;
mod property;
mod store;
mod store_conduit;

use conduit_property::ConduitProperty;
use notification_source::NotificationSource;
use store_conduit::StoreConduit;

pub use conduit::Conduit;
pub use property::Property;
pub use store::Store;

use crate::server::Encodable;
use crate::state::{EntityKey, State};

pub fn new_conduit_property(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    conduit: Box<dyn Conduit>,
) {
    let property = state
        .properties
        .insert_with_key(|key| Box::new(ConduitProperty::new(key, entity, name, conduit)));
    state.entities[entity].register_property(name, property);
}

pub fn new_store_property<T, F: 'static>(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    store_getter: F,
) where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a Store<T>, String>,
{
    new_conduit_property(
        state,
        entity,
        name,
        Box::new(StoreConduit::new(store_getter)),
    )
}

// TODO: test that new_property() adds property to entity
