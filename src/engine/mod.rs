//! The core engine, which is essentially an ECS for the game logic fused with a reactive property
//! system to talk to clients

use super::*;

mod component_key;
mod conduit;
mod element;
mod entity;
mod property;
mod state;
mod subscriber;
mod subscription_tracker;

pub use conduit::{CachingConduit, ComponentListConduit, Conduit, ElementConduit};
pub use element::Element;
pub use state::{EntityKey, NotifQueue, State};

use component_key::ComponentKey;
use entity::Entity;
use property::Property;
use subscriber::Subscriber;
use subscription_tracker::SubscriptionTracker;
