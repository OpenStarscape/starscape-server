mod caching_conduit;
/// This module contains everything concerned with shuffling data
/// between the state and connections
mod conduit;
mod notification_sink;
mod notification_source;
mod property_conduit;
mod update_source;

use caching_conduit::CachingConduit;
use notification_source::NotificationSource;
use property_conduit::PropertyConduit;

pub use conduit::Conduit;
pub use notification_sink::NotificationSink;
pub use update_source::UpdateSource;

use crate::server::Encodable;
use crate::state::{EntityKey, State};
use std::sync::{Arc, Weak};

pub fn new_conduit_property(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    conduit: Box<dyn Conduit>,
) {
    let property = Box::new(CachingConduit::new(conduit));
    state.entities.register_property(entity, name, property);
}

pub fn new_store_property<T, F: 'static>(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    store_getter: F,
) where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> F: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
    F: Clone + 'static,
{
    new_conduit_property(
        state,
        entity,
        name,
        Box::new(PropertyConduit::new(store_getter)),
    )
}

// TODO: test that new_property() adds property to entity
