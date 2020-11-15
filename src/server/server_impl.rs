use super::*;

new_key_type! {
    pub struct ConnectionKey;
}

pub struct ServerImpl {
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    _listeners: Vec<Box<dyn Listener>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
    request_tx: Sender<ServerRequest>,
    request_rx: Receiver<ServerRequest>,
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

    #[cfg(test)]
    fn get_request_tx(&self) -> &Sender<ServerRequest> {
        &self.request_tx
    }

    #[cfg(test)]
    fn get_connections(&mut self) -> &mut DenseSlotMap<ConnectionKey, Box<dyn Connection>> {
        &mut self.connections
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
        handler: &mut dyn RequestHandler,
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
                handler.set(entity, property, &value)?;
            }
            PropertyRequest::Get => {
                let value = handler.get(entity, property)?;
                eprintln!(
                    "get {}.{} returned {:?} (reply not implemented)",
                    object_id, property, value
                );
            }
            PropertyRequest::Subscribe => {
                handler.subscribe(entity, property, connection_key)?;
            }
            PropertyRequest::Unsubscribe => {
                handler.unsubscribe(entity, property, connection_key)?;
            }
        };
        Ok(())
    }

    fn process_request(&mut self, handler: &mut dyn RequestHandler, request: ServerRequest) {
        match request.request {
            ConnectionRequest::Property((obj, prop), action) => {
                if let Err(e) =
                    self.process_property_request(handler, request.connection, obj, &prop, action)
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
    fn process_requests(&mut self, handler: &mut dyn RequestHandler) {
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_build_connection(session_builder);
        }
        while let Ok(request) = self.request_rx.try_recv() {
            self.process_request(handler, request);
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
            _handle_incoming_data: Box<dyn FnMut(&[u8]) + Send>,
        ) -> Result<Box<dyn Session>, Box<dyn Error>> {
            if self.0 {
                Ok(Box::new(MockSession))
            } else {
                Err("Session builder is supposed to error for test".into())
            }
        }
    }

    struct MockConnection(EntityKey);

    impl Connection for MockConnection {
        fn property_changed(
            &self,
            _entity: EntityKey,
            _property: &str,
            _value: &Encodable,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn entity_destroyed(&self, _state: &crate::State, _entity: EntityKey) {}

        fn object_to_entity(&self, _object: ObjectId) -> Option<EntityKey> {
            Some(self.0)
        }
    }

    struct MockRequestHandler(Mutex<Vec<(String, EntityKey, String)>>);

    impl Default for MockRequestHandler {
        fn default() -> Self {
            Self(Mutex::new(Vec::new()))
        }
    }

    impl RequestHandler for MockRequestHandler {
        fn set(
            &mut self,
            entity: EntityKey,
            property: &str,
            _value: &Decodable,
        ) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push(("set".into(), entity, property.into()));
            Ok(())
        }
        fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String> {
            self.0
                .lock()
                .unwrap()
                .push(("get".into(), entity, property.into()));
            Ok(Encodable::Null)
        }
        fn subscribe(
            &mut self,
            entity: EntityKey,
            property: &str,
            _connection: ConnectionKey,
        ) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push(("subscribe".into(), entity, property.into()));
            Ok(())
        }
        fn unsubscribe(
            &mut self,
            entity: EntityKey,
            property: &str,
            _connection: ConnectionKey,
        ) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push(("unsubscribe".into(), entity, property.into()));
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
        let mut handler = MockRequestHandler::default();
        server.process_requests(&mut handler);
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
        let mut handler = MockRequestHandler::default();
        assert_eq!(server.number_of_connections(), 0);
        server.process_requests(&mut handler);
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
        let mut handler = MockRequestHandler::default();
        server.process_requests(&mut handler);
        assert_eq!(server.number_of_connections(), 0);
    }

    #[test]
    fn makes_requests() {
        let mut server = ServerImpl::new(|_| vec![]);
        let entities = mock_keys(1);
        let conn_key = server
            .get_connections()
            .insert(Box::new(MockConnection(entities[0])));
        for request in vec![
            ServerRequest::new(
                conn_key,
                ConnectionRequest::Property((1, "foo".into()), PropertyRequest::Subscribe),
            ),
            ServerRequest::new(
                conn_key,
                ConnectionRequest::Property((1, "bar".into()), PropertyRequest::Get),
            ),
        ] {
            server.get_request_tx().send(request).unwrap();
        }
        let mut handler = MockRequestHandler::default();
        server.process_requests(&mut handler);
        assert_eq!(
            *handler.0.lock().unwrap(),
            vec![
                ("subscribe".into(), entities[0], "foo".into()),
                ("get".into(), entities[0], "bar".into()),
            ]
        );
    }
}
