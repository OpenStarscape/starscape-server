//! The core engine, which is essentially an ECS for the game logic fused with a reactive property
//! system to talk to clients

use super::*;

mod component_key;
mod conduit;
mod element;
#[allow(clippy::module_inception)]
mod engine;
mod entity;
mod notif_queue;
mod property;
mod state;
mod subscriber;
mod subscription_tracker;

pub use conduit::{ComponentListConduit, Conduit, ROConduit, RWConduit};
pub use element::Element;
pub use engine::Engine;
pub use notif_queue::{NotifQueue, Notification};
pub use state::{EntityKey, State};

use component_key::ComponentKey;
use entity::Entity;
use property::Property;
use subscriber::Subscriber;
use subscription_tracker::SubscriptionTracker;
