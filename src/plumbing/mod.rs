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

use crate::{
    server::{Decodable, Encodable},
    state::{EntityKey, State},
};
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

pub fn new_store_property<T, GetFn, SetFn>(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    getter: GetFn,
    setter: SetFn,
) where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a UpdateSource<T>, String>,
    SetFn: Fn(&mut State, &Decodable) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    new_conduit_property(
        state,
        entity,
        name,
        Box::new(PropertyConduit::new(getter, setter)),
    )
}

// TODO: test that new_property() adds property to entity
