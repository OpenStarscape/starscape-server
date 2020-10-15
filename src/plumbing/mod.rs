/// This module contains everything concerned with shuffling data
/// between the state and connections
use super::*;

mod caching_conduit;
mod conduit;
mod element;
mod element_conduit;
mod subscriber;
mod subscription_tracker;

use caching_conduit::CachingConduit;
use element_conduit::ElementConduit;
use subscription_tracker::SubscriptionTracker;

pub use conduit::Conduit;
pub use element::Element;
pub use subscriber::Subscriber;

pub fn new_conduit_property(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    conduit: Box<dyn Conduit>,
) {
    let property = Box::new(CachingConduit::new(conduit));
    state.entities.register_property(entity, name, property);
}

pub fn new_element_property<T, GetFn, SetFn>(
    state: &mut State,
    entity: EntityKey,
    name: &'static str,
    getter: GetFn,
    setter: SetFn,
) where
    T: Into<Encodable> + PartialEq + Clone,
    for<'a> GetFn: Fn(&'a State) -> Result<&'a Element<T>, String>,
    SetFn: Fn(&mut State, &Decodable) -> Result<(), String>,
    GetFn: Clone + 'static,
    SetFn: Clone + 'static,
{
    new_conduit_property(
        state,
        entity,
        name,
        Box::new(ElementConduit::new(getter, setter)),
    )
}

// TODO: test that new_property() adds property to entity
