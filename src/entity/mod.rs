#[allow(clippy::module_inception)]
mod entity;
mod entity_store;
mod entity_store_impl;
mod property;
mod property_impl;

pub use entity::EntityKey;
pub use entity_store::EntityStore;
pub use property::Property;

use crate::{
    plumbing::Conduit,
    state::{BodyKey, ShipKey, State},
};
use entity::Entity;
use entity_store_impl::EntityStoreImpl;
use property_impl::PropertyImpl;
use std::sync::{Arc, Weak};
