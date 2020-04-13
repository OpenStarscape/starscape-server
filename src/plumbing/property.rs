use std::error::Error;

use crate::state::{ConnectionKey, State};

/// Represents the link between 0, 1 or more connections and a property
/// The property is generally a Store somewhere in the state
pub trait Property {
    /// Called by the engine
    /// Should retrieve a value from the state and send it to all subscribed connections
    fn send_updates(&self, state: &State) -> Result<(), Box<dyn Error>>;
    /// Causes the connection to start getting updates
    /// The error string should be appropriate to send to clients
    fn subscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String>;
    /// Stops the flow of updates to the given connection
    /// The error string should be appropriate to send to clients
    fn unsubscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String>;
}
