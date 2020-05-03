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
    /// Conceptually a set, but due to characteristics (few elements, infrequent modification)
    /// a Vec should perform better in most cases
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
        if subscribers.contains(&connection) {
            // TODO: preserve entity and convert it to an object ID in the connection
            // Will need a new error format for this
            Err(format!(
                "already subscribed to {:?}.{}",
                self.entity, self.name
            ))
        } else {
            if subscribers.is_empty() {
                self.conduit.connect(state, self.self_key)?;
            }
            subscribers.push(connection);
            Ok(())
        }
    }

    fn unsubscribe(&self, state: &State, connection: ConnectionKey) -> Result<(), String> {
        let mut subscribers = self.subscribers.write().unwrap();
        match subscribers.iter().position(|key| *key == connection) {
            // TODO: preserve entity and convert it to an object ID in the connection
            // Will need a new error format for this
            None => Err(format!("not subscribed to {:?}.{}", self.entity, self.name)),
            Some(i) => {
                subscribers.swap_remove(i);
                if subscribers.is_empty() {
                    self.conduit.disconnect(state, self.self_key)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::state::mock_keys;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct MockConnection {
        log: Vec<(EntityKey, String, Value)>,
    }

    impl MockConnection {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self { log: Vec::new() }))
        }
    }

    impl Connection for Rc<RefCell<MockConnection>> {
        fn property_changed(
            &self,
            entity: EntityKey,
            property: &str,
            value: &Value,
        ) -> Result<(), Box<dyn Error>> {
            self.borrow_mut()
                .log
                .push((entity, property.to_owned(), value.clone()));
            Ok(())
        }

        fn entity_destroyed(&self, _state: &State, _entity: EntityKey) {
            panic!("Unexpected call");
        }

        fn subscribe_to(&self, _state: &State, _entity: EntityKey, _property: &str) {
            panic!("Unexpected call");
        }
    }

    struct MockConduit {
        value_to_get: Result<Value, String>,
        connected: Option<PropertyKey>,
    }

    impl MockConduit {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                value_to_get: Err("No value yet".to_owned()),
                connected: None,
            }))
        }
    }

    impl Conduit for Rc<RefCell<MockConduit>> {
        fn get_value(&self, _sate: &State) -> Result<Value, String> {
            self.borrow().value_to_get.clone()
        }

        fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
            panic!("Unexpected call");
        }

        fn connect(&self, _state: &State, property: PropertyKey) -> Result<(), String> {
            assert!(self.borrow().connected.is_none());
            self.borrow_mut().connected = Some(property);
            Ok(())
        }

        fn disconnect(&self, _state: &State, property: PropertyKey) -> Result<(), String> {
            assert!(self.borrow().connected == Some(property));
            self.borrow_mut().connected = None;
            Ok(())
        }
    }

    fn setup_without_connection() -> (
        State,
        PropertyKey,
        Vec<ConnectionKey>,
        Rc<RefCell<MockConduit>>,
        ConduitProperty,
    ) {
        let state = State::new();
        let entity_keys = mock_keys(1);
        let conn_keys = mock_keys(3);
        let prop_keys = mock_keys(1);
        let conduit = MockConduit::new();
        let property = ConduitProperty::new(
            prop_keys[0],
            entity_keys[0],
            "foo",
            Box::new(conduit.clone()),
        );
        (state, prop_keys[0], conn_keys, conduit, property)
    }

    fn setup_with_connection() -> (
        State,
        EntityKey,
        Rc<RefCell<MockConnection>>,
        ConnectionKey,
        Rc<RefCell<MockConduit>>,
        ConduitProperty,
    ) {
        let mut state = State::new();
        let entity_keys = mock_keys(1);
        let mock_conn = MockConnection::new();
        let conn_key = state.connections.insert(Box::new(mock_conn.clone()));
        let prop_keys = mock_keys(1);
        let conduit = MockConduit::new();
        let property = ConduitProperty::new(
            prop_keys[0],
            entity_keys[0],
            "foo",
            Box::new(conduit.clone()),
        );
        (
            state,
            entity_keys[0],
            mock_conn,
            conn_key,
            conduit,
            property,
        )
    }

    #[test]
    fn first_subscription_connects_conduit() {
        let (state, prop_key, conn_keys, conduit, property) = setup_without_connection();
        assert_eq!(conduit.borrow().connected, None);
        property
            .subscribe(&state, conn_keys[0])
            .expect("failed to subscribe");
        assert_eq!(conduit.borrow().connected, Some(prop_key));
    }

    #[test]
    fn subsequent_subscriptions_do_not_connect_conduit() {
        let (state, prop_key, conn_keys, conduit, property) = setup_without_connection();
        for c in conn_keys {
            // mock conduit should panic if subscribed multiple times
            property.subscribe(&state, c).expect("failed to subscribe");
        }
        assert_eq!(conduit.borrow().connected, Some(prop_key));
    }

    #[test]
    fn does_not_disconnect_on_first_unsubscribe() {
        let (state, prop_key, conn_keys, conduit, property) = setup_without_connection();
        for c in &conn_keys {
            // mock conduit should panic if subscribed multiple times
            property
                .subscribe(&state, c.clone())
                .expect("failed to subscribe");
        }
        property
            .unsubscribe(&state, conn_keys[0])
            .expect("failed to unsubscribe");
        assert_eq!(conduit.borrow().connected, Some(prop_key));
    }

    #[test]
    fn removing_only_subscription_disconnects_conduit() {
        let (state, prop_key, conn_keys, conduit, property) = setup_without_connection();
        property.subscribers.write().unwrap().push(conn_keys[0]);
        conduit.borrow_mut().connected = Some(prop_key);
        property
            .unsubscribe(&state, conn_keys[0])
            .expect("failed to unsubscribe");
        assert_eq!(conduit.borrow().connected, None);
    }

    #[test]
    fn removing_all_subscriptions_disconnects_conduit() {
        let (state, _, conn_keys, conduit, property) = setup_without_connection();
        for c in &conn_keys {
            // mock conduit should panic if subscribed multiple times
            property
                .subscribe(&state, c.clone())
                .expect("failed to subscribe");
        }
        for c in &conn_keys {
            // mock conduit should panic if unsubscribed multiple times
            property
                .unsubscribe(&state, c.clone())
                .expect("failed to unsubscribe");
        }
        assert_eq!(conduit.borrow().connected, None);
    }

    #[test]
    fn single_connection_subscribing_twice_errors() {
        let (state, _, conn, _, property) = setup_without_connection();
        property
            .subscribe(&state, conn[0])
            .expect("failed to subscribe");
        assert!(property.subscribe(&state, conn[0]).is_err());
    }

    #[test]
    fn unsubscribing_with_connection_not_subscribed_errors() {
        let (state, _, conn, _, property) = setup_without_connection();
        assert!(property.unsubscribe(&state, conn[0]).is_err());
    }

    #[test]
    fn when_updated_sends_correct_data() {
        let (state, entity_key, mock_conn, conn_key, conduit, property) = setup_with_connection();
        property.subscribers.write().unwrap().push(conn_key);
        conduit.borrow_mut().value_to_get = Ok(Value::Integer(42));
        property
            .send_updates(&state)
            .expect("failed to send updates");
        assert_eq!(
            mock_conn.borrow().log,
            vec![(entity_key, "foo".to_owned(), Value::Integer(42))]
        );
    }

    #[test]
    fn sends_multiple_values_on_change() {
        let (state, entity_key, mock_conn, conn_key, conduit, property) = setup_with_connection();
        property.subscribers.write().unwrap().push(conn_key);
        conduit.borrow_mut().value_to_get = Ok(Value::Integer(42));
        property
            .send_updates(&state)
            .expect("failed to send updates");
        conduit.borrow_mut().value_to_get = Ok(Value::Integer(69));
        property
            .send_updates(&state)
            .expect("failed to send updates");
        assert_eq!(
            mock_conn.borrow().log,
            vec![
                (entity_key, "foo".to_owned(), Value::Integer(42)),
                (entity_key, "foo".to_owned(), Value::Integer(69)),
            ]
        );
    }

    #[test]
    fn does_not_send_same_data_twice() {
        let (state, entity_key, mock_conn, conn_key, conduit, property) = setup_with_connection();
        property.subscribers.write().unwrap().push(conn_key);
        conduit.borrow_mut().value_to_get = Ok(Value::Integer(42));
        property
            .send_updates(&state)
            .expect("failed to send updates");
        property
            .send_updates(&state)
            .expect("failed to send updates");
        assert_eq!(
            mock_conn.borrow().log,
            vec![(entity_key, "foo".to_owned(), Value::Integer(42))]
        );
    }
}
