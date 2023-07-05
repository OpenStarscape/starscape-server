//! The core engine, which is essentially an ECS for the game logic fused with a reactive property
//! system to talk to clients

use super::*;

mod conduit;
mod element;
mod engine;
mod engine_config;
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
pub use engine_config::*;
pub use id::{GenericId, Id};
pub use notif_queue::{NotifQueue, Notification};
pub use object::{MemberType, Object};
pub use signal::Signal;
pub use state::{HasCollection, State};
pub use subscribable::Subscribable;
pub use subscriber::Subscriber;
pub use subscriber_list::SubscriberList;
pub use sync_subscriber_list::SyncSubscriberList;
pub use value::Value;

use conduit::*;
use signal::SignalsDontTakeInputSilly;
use subscription_impl::SubscriptionImpl;
