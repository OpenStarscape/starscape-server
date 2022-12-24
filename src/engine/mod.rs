//! The core engine, which is essentially an ECS for the game logic fused with a reactive property
//! system to talk to clients

use super::*;

mod component_key;
mod conduit;
mod element;
mod engine;
mod entity;
mod id;
mod notif_queue;
mod object;
mod signal;
mod state;
mod subscribable;
mod subscriber;
mod subscriber_list;
mod subscription_impl;
mod sync_subscriber_list;
mod value;

pub use conduit::{
    ActionConduit, ComponentListConduit, Conduit, ConstConduit, ROConduit, RWConduit,
    ReadOnlyPropSetType,
};
pub use element::Element;
pub use engine::Engine;
pub use id::{GenericId, Id};
pub use notif_queue::{NotifQueue, Notification};
pub use object::Object;
pub use signal::Signal;
pub use state::{EntityKey, HasCollection, State};
pub use subscribable::Subscribable;
pub use subscriber::Subscriber;
pub use subscriber_list::SubscriberList;
pub use sync_subscriber_list::SyncSubscriberList;
pub use value::Value;

use component_key::ComponentKey;
use conduit::*;
use entity::Entity;
use signal::SignalsDontTakeInputSilly;
use subscription_impl::SubscriptionImpl;
