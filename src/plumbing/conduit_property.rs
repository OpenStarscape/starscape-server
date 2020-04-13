use std::error::Error;
use std::sync::{Mutex, RwLock};

use super::{Conduit, Property};
use crate::connection::Value;
use crate::state::{ConnectionKey, EntityKey, PropertyKey, State};

/// The default property implementation
pub struct ConduitProperty {
    self_key: PropertyKey,
    entity: EntityKey,
    name: &'static str,
    cached_value: Mutex<Value>,
    conduit: Box<dyn Conduit>,
    subscribers: RwLock<Vec<ConnectionKey>>,
}

impl ConduitProperty {
    pub fn new(
        self_key: PropertyKey,
        entity: EntityKey,
        name: &'static str,
        conduit: Box<dyn Conduit>,
    ) -> Self {
        Self {
            self_key,
            entity,
            name,
            cached_value: Mutex::new(Value::Null),
            conduit,
            subscribers: RwLock::new(Vec::new()),
        }
    }
}

impl Property for ConduitProperty {
    fn send_updates(&self, state: &State) -> Result<(), Box<dyn Error>> {
        let value = self.conduit.get_value(state)?;
        let mut cached = self
            .cached_value
            .lock()
            .expect("Failed to lock cached value mutex");
        if *cached != value {
            *cached = value.clone();
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
            self.conduit.connect(state, self.self_key)?;
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
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::state::{mock_keys, PropertyKey};
    use slotmap::Key;
    use std::sync::Arc;

    struct MockEntity {}

    impl Entity for MockEntity {
        fn add_property(&mut self, _name: &'static str, _key: PropertyKey) {}
        fn property(&self, name: &str) -> Result<PropertyKey, String> {
            Err("Not implemented".to_owned())
        }
        fn destroy(&mut self, _state: &mut State) {}
    }

    /*
    #[test]
    fn when_updated_sends_correct_entity_key() {
        let mut state = State::new();
        let e = mock_keys(1);
        let prop = Arc::new(Property::new(7));
        let conduit = PropertyConduit::new(ConduitKey::null(), e[0], "foo", {
            let p = prop.clone();
            move |state| &*p
        });
        panic!("Implementation not finished");
    }
    */

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
