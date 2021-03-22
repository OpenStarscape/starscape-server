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
mod signal;
mod state;
mod subscribable;
mod subscriber;
mod subscriber_list;
mod subscription;
mod sync_subscriber_list;
mod value;

pub use conduit::*;
pub use element::Element;
pub use engine::Engine;
pub use notif_queue::{NotifQueue, Notification};
pub use signal::Signal;
pub use state::{EntityKey, State};
pub use subscriber::Subscriber;
pub use subscriber_list::SubscriberList;
pub use sync_subscriber_list::SyncSubscriberList;
pub use value::Value;

use component_key::ComponentKey;
use conduit::*;
use entity::Entity;
use signal::SignalsDontTakeInputSilly;
use subscribable::Subscribable;
use subscription::Subscription;
