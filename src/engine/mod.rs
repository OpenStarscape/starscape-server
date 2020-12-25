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
mod event_element;
mod notif_queue;
mod state;
mod subscriber;
mod subscriber_list;
mod subscription;

pub use conduit::{ActionConduit, ComponentListConduit, Conduit, ROConduit, RWConduit};
pub use element::Element;
pub use engine::Engine;
pub use event_element::EventElement;
pub use notif_queue::{NotifQueue, Notification};
pub use state::{EntityKey, State};
pub use subscriber::Subscriber;

use component_key::ComponentKey;
use conduit::*;
use conduit_subscriber_list::ConduitSubscriberList;
use entity::Entity;
use event_element::EventsDontTakeInputSilly;
use subscriber_list::SubscriberList;
use subscription::Subscription;
