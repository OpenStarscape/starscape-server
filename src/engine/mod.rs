//! The core engine, which is essentially an ECS for the game logic fused with a reactive property
//! system to talk to clients

use super::*;

mod component_key;
mod conduit;
mod conduit_subscriber_list;
mod element;
#[allow(clippy::module_inception)]
mod engine;
mod entity;
mod notif_queue;
mod signal;
mod state;
mod subscriber;
mod subscriber_list;
mod subscription;
mod value;

pub use conduit::{ActionConduit, ComponentListConduit, Conduit, ROConduit, RWConduit};
pub use element::Element;
pub use engine::Engine;
pub use notif_queue::{NotifQueue, Notification};
pub use signal::Signal;
pub use state::{EntityKey, State};
pub use subscriber::Subscriber;
pub use value::{Decoded, Encodable};

use component_key::ComponentKey;
use conduit::*;
use conduit_subscriber_list::ConduitSubscriberList;
use entity::Entity;
use signal::SignalsDontTakeInputSilly;
use subscriber_list::SubscriberList;
use subscription::Subscription;
