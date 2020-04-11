use std::sync::{Mutex, RwLock};

use crate::connection::Value;
use crate::property::Property;
use crate::state::{ConduitKey, ConnectionKey, EntityKey, State};

/// The link between a single property somewhere in the state and client connections
pub trait Conduit {
    /// Called by the engine
    /// Conduit should retrieve a value from the state and send it to all subscribed connections
    fn send_updates(&self, state: &State) -> Result<(), String>;
    /// Causes the connection to start getting updates
    fn subscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String>;
    /// Stops the flow of updates to the given connection
    fn unsubscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String>;
}

/// A conduit that passes on a Property value without changing it
pub struct PropertyConduit<T, F> {
    conduit: ConduitKey,
    entity: EntityKey,
    name: &'static str,
    cached_value: Mutex<Option<T>>,
    property: F,
    subscribers: RwLock<Vec<ConnectionKey>>,
}

impl<T, F> PropertyConduit<T, F>
where
    for<'a> F: Fn(&'a State) -> &'a Property<T>,
    T: Clone + PartialEq + Into<Value>,
{
    pub fn new(conduit: ConduitKey, entity: EntityKey, name: &'static str, property: F) -> Self {
        Self {
            conduit,
            entity,
            name,
            cached_value: Mutex::new(None),
            property,
            subscribers: RwLock::new(Vec::new()),
        }
    }
}

impl<T, F> Conduit for PropertyConduit<T, F>
where
    for<'a> F: Fn(&'a State) -> &'a Property<T>,
    T: Clone + PartialEq + Into<Value>,
{
    fn send_updates(&self, state: &State) -> Result<(), String> {
        let property = (self.property)(state);
        let mut cached = self
            .cached_value
            .lock()
            .expect("Failed to lock cached value mutex");
        if cached.is_none() || cached.as_ref().unwrap() != property.value() {
            *cached = Some(property.value().clone());
            let value: Value = (property.value().clone()).into();
            let subscribers = self.subscribers.read().expect("Failed to read subscribers");
            for connection_key in &*subscribers {
                if let Some(connection) = state.connections.get(*connection_key) {
                    if let Err(e) = connection.property_changed(self.entity, self.name, &value) {
                        eprintln!(
                            "Error updating property {:?}.{}: {}",
                            self.entity, self.name, e
                        );
                    }
                } else {
                    eprintln!(
                        "Error updating property {:?}.{}: connection {:?} has died",
                        self.entity, self.name, connection_key
                    );
                }
            }
        }
        Ok(())
    }

    fn subscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String> {
        let mut subscribers = self.subscribers.write().unwrap();
        // TODO: error checking
        if subscribers.is_empty() {
            (self.property)(state).connect(self.conduit);
        }
        subscribers.push(connection);
        Ok(())
    }

    fn unsubscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String> {
        // TODO
        Ok(())
    }
}

// TODO: make connection generic so testing is easier
#[cfg(test)]
mod property_conduit_tests {
    use super::*;
    use crate::entity::Entity;
    use crate::state::ConduitKey;

    struct MockEntity {}

    impl Entity for MockEntity {
        fn add_conduit(&mut self, _name: &'static str, _conduit: ConduitKey) {}
        fn conduit(&self, property: &str) -> Result<ConduitKey, String> {
            Err("Not implemented".to_owned())
        }
        fn destroy(&mut self, _state: &mut State) {}
    }

    #[test]
    fn when_updated_sends_correct_entity_key() {
        let mut state = State::new();
        let e_key = state.entities.insert(Box::new(MockEntity {}));
        let prop = Property::new(7);
        //let _conduit = PropertyConduit::new(e_key, "foo", move |_| &'a prop);
        panic!("Implementation not finished");
    }

    #[test]
    fn when_updated_sends_correct_name() {
        panic!("Test not implemented");
    }

    #[test]
    fn when_updated_sends_correct_value() {
        panic!("Test not implemented");
    }

    #[test]
    fn get_value_is_correct() {
        panic!("Test not implemented");
    }

    #[test]
    fn get_value_is_correct_after_thare_was_a_subscriver() {
        panic!("Test not implemented");
    }

    #[test]
    fn does_not_update_subscribers_when_unchaged() {
        panic!("Test not implemented");
    }

    #[test]
    fn first_subscriber_connects_to_property() {
        panic!("Test not implemented");
    }

    #[test]
    fn subsequent_subscribers_do_not_trigger_property_connect() {
        panic!("Test not implemented");
    }

    #[test]
    fn removing_single_subscriber_disconnects_from_property() {
        panic!("Test not implemented");
    }

    #[test]
    fn removing_all_subscribers_discpnnects_from_property() {
        panic!("Test not implemented");
    }

    #[test]
    fn single_connection_subscribing_twice_errors() {
        panic!("Test not implemented");
    }

    #[test]
    fn unsubscribing_with_connection_not_subscribed_errors() {
        panic!("Test not implemented");
    }
}
