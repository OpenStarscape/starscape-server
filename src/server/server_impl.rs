use slotmap::DenseSlotMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::*;
use crate::EntityKey;

new_key_type! {
    pub struct ConnectionKey;
}

pub struct ServerImpl {
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    _listeners: Vec<Box<dyn Listener>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
    request_tx: Sender<Request>,
    request_rx: Receiver<Request>,
}

impl ServerImpl {
    pub fn new<F>(build_listeners: F) -> Self
    where
        F: Fn(Sender<Box<dyn SessionBuilder>>) -> Vec<Box<dyn Listener>>,
    {
        let (new_session_tx, new_session_rx) = channel();
        let (request_tx, request_rx) = channel();
        ServerImpl {
            connections: DenseSlotMap::with_key(),
            _listeners: build_listeners(new_session_tx),
            new_session_rx,
            request_tx,
            request_rx,
        }
    }
}

impl ServerImpl {
    fn try_build_connection(&mut self, builder: Box<dyn SessionBuilder>) {
        eprintln!("New session: {:?}", builder);
        // hack to get around slotmap only giving us a key after creation
        let key = self.connections.insert(Box::new(()));
        let (encoder, decoder) = json_protocol_impls();
        match ConnectionImpl::new(key, encoder, decoder, builder, self.request_tx.clone()) {
            Ok(c) => {
                self.connections[key] = Box::new(c);
            }
            Err(e) => {
                self.connections.remove(key);
                eprintln!("Error building connection: {}", e);
            }
        }
    }

    fn process_property_request(
        &self,
        state: &mut dyn ServerState,
        connection_key: ConnectionKey,
        object_id: ObjectId,
        property: &str,
        action: PropertyRequest,
    ) -> Result<(), String> {
        let connection = self
            .connections
            .get(connection_key)
            .ok_or("request on invalid connection")?;
        let entity = connection
            .object_to_entity(object_id)
            .ok_or("object not known to connection")?;
        match action {
            PropertyRequest::Set(value) => {
                state.set(entity, property, value)?;
            }
            PropertyRequest::Get => {
                let value = state.get(entity, property)?;
                eprintln!(
                    "get {}.{} returned {:?} (reply not implemented)",
                    object_id, property, value
                );
            }
            PropertyRequest::Subscribe => {
                state.subscribe(entity, property, connection_key)?;
            }
            PropertyRequest::Unsubscribe => {
                state.unsubscribe(entity, property, connection_key)?;
            }
        };
        Ok(())
    }

    fn process_request(&mut self, state: &mut dyn ServerState, request: Request) {
        match request.request {
            ConnectionRequest::Property((obj, prop), action) => {
                if let Err(e) =
                    self.process_property_request(state, request.connection, obj, &prop, action)
                {
                    eprintln!("Error processing request: {:?}", e);
                }
            }
            ConnectionRequest::Close => {
                eprintln!("Closing connection {:?}", request.connection);
                if self.connections.remove(request.connection).is_none() {
                    eprintln!("Invalid connection closed: {:?}", request.connection);
                }
            }
        };
    }
}

impl PropertyUpdateSink for ServerImpl {
    fn property_changed(
        &self,
        connection_key: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(connection) = self.connections.get(connection_key) {
            connection.property_changed(entity, property, &value)?;
            Ok(())
        } else {
            Err(format!("connection {:?} has died", connection_key).into())
        }
    }
}

impl Server for ServerImpl {
    fn apply_updates(&mut self, state: &mut dyn ServerState) {
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_build_connection(session_builder);
        }
        while let Ok(request) = self.request_rx.try_recv() {
            self.process_request(state, request);
        }
    }

    fn number_of_connections(&self) -> usize {
        self.connections.len()
    }

    fn property_update_sink(&self) -> &dyn PropertyUpdateSink {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, Weak};

    #[derive(Debug)]
    struct MockSession;

    impl Session for MockSession {
        fn send(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
            panic!("Unecpected call");
        }
    }

    #[derive(Debug)]
    struct MockSessionBuilder(bool);

    impl SessionBuilder for MockSessionBuilder {
        fn build(
            self: Box<Self>,
            _handle_incoming_data: Box<dyn FnMut(&[u8]) -> () + Send>,
        ) -> Result<Box<dyn Session>, Box<dyn Error>> {
            if self.0 {
                Ok(Box::new(MockSession))
            } else {
                Err("Session builder is supposed to error for test".into())
            }
        }
    }

    struct MockServerState;

    impl ServerState for MockServerState {
        fn set(
            &mut self,
            _entity: EntityKey,
            _property: &str,
            _value: Decodable,
        ) -> Result<(), String> {
            Ok(())
        }
        fn get(&self, _entity: EntityKey, _property: &str) -> Result<Encodable, String> {
            Ok(Encodable::Null)
        }
        fn subscribe(
            &mut self,
            _entity: EntityKey,
            _property: &str,
            _connection: ConnectionKey,
        ) -> Result<(), String> {
            Ok(())
        }
        fn unsubscribe(
            &mut self,
            _entity: EntityKey,
            _property: &str,
            _connection: ConnectionKey,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn builds_and_holds_listeners() {
        impl Listener for Arc<()> {}
        let weak = {
            let (weak, _server) = {
                let arc = Arc::new(());
                let weak: Weak<()> = Arc::downgrade(&arc);
                let server = ServerImpl::new(|_| vec![Box::new(arc.clone())]);
                (weak, server)
            };
            assert_eq!(weak.strong_count(), 1);
            weak
        };
        assert_eq!(weak.strong_count(), 0);
    }

    #[test]
    fn has_no_connections_by_default() {
        let mut server = ServerImpl::new(|_| vec![]);
        assert_eq!(server.number_of_connections(), 0);
        let mut state = MockServerState {};
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 0);
    }

    #[test]
    fn can_create_connection() {
        let new_session_tx = Mutex::new(None);
        let mut server = ServerImpl::new(|tx| {
            *new_session_tx.lock().unwrap() = Some(tx);
            vec![]
        });
        let builder = Box::new(MockSessionBuilder(true));
        new_session_tx
            .lock()
            .unwrap()
            .as_ref()
            .expect("new_session_tx not set")
            .send(builder)
            .expect("failed to send connection builder");
        let mut state = MockServerState {};
        assert_eq!(server.number_of_connections(), 0);
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 1);
    }

    #[test]
    fn does_not_create_connection_when_building_session_fails() {
        let new_session_tx = Mutex::new(None);
        let mut server = ServerImpl::new(|tx| {
            *new_session_tx.lock().unwrap() = Some(tx);
            vec![]
        });
        // False means building session will fail vvvvv
        let builder = Box::new(MockSessionBuilder(false));
        new_session_tx
            .lock()
            .unwrap()
            .as_ref()
            .expect("new_session_tx not set")
            .send(builder)
            .expect("failed to send connection builder");
        let mut state = MockServerState {};
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 0);
    }
}
