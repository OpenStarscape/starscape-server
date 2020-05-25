#[allow(clippy::module_inception)]
mod entity;
mod entity_store;

pub use entity_store::{EntityKey, EntityStore};

use entity::Entity;
