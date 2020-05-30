#[allow(clippy::module_inception)]
mod entity;
mod entity_store;
mod entity_store_impl;

pub use entity::EntityKey;
pub use entity_store::EntityStore;

use entity::Entity;
use entity_store_impl::EntityStoreImpl;
