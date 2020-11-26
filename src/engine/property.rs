use super::*;
use std::collections::hash_map::Entry;

pub trait Property {
    fn get_value(&self, state: &State) -> Result<Encodable, String>;
    fn set_value(&self, state: &mut State, value: &Decodable) -> Result<(), String>;
    fn subscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String>;
    fn unsubscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String>;
    fn finalize(&self, state: &State);
}

impl dyn Property {
    pub fn new(
        entity: EntityKey,
        name: &'static str,
        conduit: Box<dyn Conduit>,
    ) -> Arc<dyn Property> {
        Arc::new(PropertyImpl::new(entity, name, conduit))
    }
}

struct ConnectionData {
    connection: ConnectionKey,
    entity: EntityKey,
    property_name: &'static str,
    conduit: Box<dyn Conduit>,
}

impl Subscriber for ConnectionData {
    fn notify(&self, state: &State, sink: &dyn PropertyUpdateSink) -> Result<(), Box<dyn Error>> {
        let value = self.conduit.get_value(state)?;
        sink.property_changed(self.connection, self.entity, self.property_name, &value)
            .map_err(|e| {
                format!(
                    "error sending update for {:?}.{}: {}",
                    self.entity, self.property_name, e
                )
                .into()
            })
    }
}

struct PropertyImpl {
    entity: EntityKey,
    name: &'static str,
    conduit: Box<dyn Conduit>,
    subscriptions: Mutex<HashMap<ConnectionKey, Arc<ConnectionData>>>,
}

impl PropertyImpl {
    pub fn new(entity: EntityKey, name: &'static str, conduit: Box<dyn Conduit>) -> Self {
        PropertyImpl {
            entity,
            name,
            conduit,
            subscriptions: Mutex::new(HashMap::new()),
        }
    }
}

impl Property for PropertyImpl {
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        self.conduit.get_value(state)
    }

    fn set_value(&self, state: &mut State, value: &Decodable) -> Result<(), String> {
        self.conduit.set_value(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String> {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .expect("failed to lock subscriptions in PropertyImpl.subscribe()");
        match subscriptions.entry(subscriber) {
            Entry::Occupied(_) => Err(format!(
                "{:?} is already subscribed to {:?}.{}",
                subscriber, self.entity, self.name
            )),
            Entry::Vacant(entry) => {
                let conn_prop = Arc::new(ConnectionData {
                    connection: subscriber,
                    entity: self.entity,
                    property_name: self.name,
                    conduit: self.conduit.clone(),
                });
                let notif_sink: Arc<dyn Subscriber> = conn_prop.clone();
                self.conduit.subscribe(state, &notif_sink)?;
                entry.insert(conn_prop);
                Ok(())
            }
        }
    }

    fn unsubscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String> {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .expect("failed to lock subscriptions in PropertyImpl.unsubscribe()");
        match subscriptions.remove(&subscriber) {
            None => Err(format!(
                "{:?} is not subscribed to {:?}.{}",
                subscriber, self.entity, self.name
            )),
            Some(conn_prop) => {
                let conn_prop: Arc<dyn Subscriber> = conn_prop;
                self.conduit
                    .unsubscribe(state, &Arc::downgrade(&conn_prop))?;
                Ok(())
            }
        }
    }

    fn finalize(&self, state: &State) {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .expect("failed to lock subscriptions in PropertyImpl.finalize()");
        for (_conn, property) in subscriptions.drain() {
            let sink = Arc::downgrade(&property) as Weak<dyn Subscriber>;
            if let Err(e) = self.conduit.unsubscribe(state, &sink) {
                error!(
                    "failed to unsubscribe property from conduit during property finalize: {}",
                    e
                );
            }
        }
    }
}

impl Drop for PropertyImpl {
    fn drop(&mut self) {
        let subscriptions = self
            .subscriptions
            .lock()
            .expect("failed to lock subscriptions in PropertyImpl.drop()");
        if !subscriptions.is_empty() {
            error!(
                "PropertyImpl not finalized before drop. {} subscriptions left.",
                subscriptions.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc, sync::Weak};

    struct MockConduit {
        value_to_get: Result<Encodable, String>,
        subscribed: HashMap<*const (), Weak<dyn Subscriber>>,
    }

    impl MockConduit {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                value_to_get: Err("no Encodable yet".to_owned()),
                subscribed: HashMap::new(),
            }))
        }
    }

    impl Conduit for Rc<RefCell<MockConduit>> {
        fn get_value(&self, _sate: &State) -> Result<Encodable, String> {
            self.borrow().value_to_get.clone()
        }

        fn set_value(&self, _state: &mut State, _value: &Decodable) -> Result<(), String> {
            panic!("unexpected call");
        }

        fn subscribe(
            &self,
            _state: &State,
            subscriber: &Arc<dyn Subscriber>,
        ) -> Result<(), String> {
            let ptr = subscriber.thin_ptr();
            assert!(
                self.borrow_mut()
                    .subscribed
                    .insert(ptr, Arc::downgrade(subscriber))
                    .is_none(),
                "subscriber {:?} already subscribed",
                ptr
            );
            Ok(())
        }

        fn unsubscribe(
            &self,
            _state: &State,
            subscriber: &Weak<dyn Subscriber>,
        ) -> Result<(), String> {
            let ptr = subscriber.thin_ptr();
            assert!(
                self.borrow_mut().subscribed.remove(&ptr).is_some(),
                "unsubscriber {:?} not subscribed",
                ptr
            );
            Ok(())
        }
    }

    struct MockPropertyUpdateSink(RefCell<Vec<(ConnectionKey, EntityKey, String, Encodable)>>);

    impl MockPropertyUpdateSink {
        fn new() -> Self {
            Self(RefCell::new(Vec::new()))
        }
    }

    impl PropertyUpdateSink for MockPropertyUpdateSink {
        fn property_changed(
            &self,
            connection: ConnectionKey,
            entity: EntityKey,
            property: &str,
            value: &Encodable,
        ) -> Result<(), Box<dyn Error>> {
            self.0
                .borrow_mut()
                .push((connection, entity, property.to_owned(), value.clone()));
            Ok(())
        }
    }

    fn setup() -> (
        State,
        PropertyImpl,
        Rc<RefCell<MockConduit>>,
        Vec<ConnectionKey>,
    ) {
        let entity_keys = mock_keys(1);
        let mock_conduit = MockConduit::new();
        (
            State::new(),
            PropertyImpl::new(entity_keys[0], "foo", Box::new(mock_conduit.clone())),
            mock_conduit,
            mock_keys(3),
        )
    }

    #[test]
    fn conduit_gets_subscribed_to() {
        let (state, property, conduit, conns) = setup();
        property
            .subscribe(&state, conns[0])
            .expect("failed to subscribe");
        assert_eq!(conduit.borrow().subscribed.len(), 1);
    }

    #[test]
    fn subscribing_multiple_connections_works() {
        let (state, property, conduit, conns) = setup();
        for conn in &conns {
            // Mock conduit will panic if there's any funny business
            property
                .subscribe(&state, *conn)
                .expect("failed to subscribe");
        }
        assert_eq!(conduit.borrow().subscribed.len(), conns.len());
    }

    #[test]
    fn conduit_gets_unsubscribed_from_on_unsub() {
        let (state, property, conduit, conns) = setup();
        property
            .subscribe(&state, conns[0])
            .expect("failed to subscribe");
        property
            .unsubscribe(&state, conns[0])
            .expect("failed to subscribe");
        assert_eq!(conduit.borrow().subscribed.len(), 0);
    }

    #[test]
    fn conduit_gets_unsubscribed_from_on_finalize() {
        let (state, property, conduit, conns) = setup();
        {
            let property = property;
            property
                .subscribe(&state, conns[0])
                .expect("failed to subscribe");
            property.finalize(&state);
        }
        assert_eq!(conduit.borrow().subscribed.len(), 0);
    }

    #[test]
    fn subscribing_the_same_connection_multiple_times_errors() {
        let (state, property, _, conns) = setup();
        property
            .subscribe(&state, conns[0])
            .expect("failed to subscribe");
        assert!(property.subscribe(&state, conns[0]).is_err());
    }

    #[test]
    fn sends_update_to_subscribed_conneection() {
        let entities = mock_keys(1);
        let conduit = MockConduit::new();
        let state = State::new();
        let prop = "foo";
        let property = PropertyImpl::new(entities[0], prop, Box::new(conduit.clone()));
        let conns = mock_keys(1);
        let prop_sink = MockPropertyUpdateSink::new();
        let value = Encodable::Integer(42);
        conduit.borrow_mut().value_to_get = Ok(value.clone());
        property
            .subscribe(&state, conns[0])
            .expect("failed to subscribe");
        for sink in conduit.borrow().subscribed.values() {
            sink.upgrade()
                .expect("subscribed sink null")
                .notify(&state, &prop_sink)
                .expect("failed to notify");
        }
        assert_eq!(
            *prop_sink.0.borrow(),
            vec![(conns[0], entities[0], prop.to_owned(), value)]
        );
    }
}
