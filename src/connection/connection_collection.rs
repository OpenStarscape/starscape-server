use super::*;

/// Allows sending property updates and other messages to clients. Implemented by
/// ConnectionCollection and used by the engine.
pub trait OutboundMessageHandler {
    fn property_update(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
}

/// Processes requests from a client. Implemented by State in the engine and used by
/// ConnectionCollection.
pub trait InboundMessageHandler {
    fn set(&mut self, entity: EntityKey, property: &str, value: &Decodable) -> Result<(), String>;
    fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String>;
    fn subscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String>;
    fn unsubscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String>;
}

/// Holds all the active connections for a game. process_requests() should be called by the game
/// once per network tick.
pub struct ConnectionCollection {
    root_entity: EntityKey,
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    new_session_tx: Sender<Box<dyn SessionBuilder>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
    request_tx: Sender<Request>,
    request_rx: Receiver<Request>,
}

impl ConnectionCollection {
    pub fn new(root_entity: EntityKey) -> Self {
        let connections = DenseSlotMap::with_key();
        let (new_session_tx, new_session_rx) = channel();
        let (request_tx, request_rx) = channel();
        Self {
            root_entity,
            connections,
            new_session_tx,
            new_session_rx,
            request_tx,
            request_rx,
        }
    }

    /// When a SessionBuilder is sent over this channel, it will be used to create a new connection
    pub fn new_session_sender(&self) -> Sender<Box<dyn SessionBuilder>> {
        self.new_session_tx.clone()
    }

    /// Handle incoming connection requests and messages from clients on the current thread. Should
    /// be called at the start of each network tick.
    pub fn process_inbound_messages(&mut self, handler: &mut dyn InboundMessageHandler) {
        // First, build sessions for any new clients that are trying to connect
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_to_build_connection(session_builder);
        }
        // Then process pending requests
        while let Ok(request) = self.request_rx.try_recv() {
            info!("got request: {:?}", request);
            self.request(handler, request);
        }
    }

    fn try_to_build_connection(&mut self, builder: Box<dyn SessionBuilder>) {
        info!("new session: {:?}", builder);

        // DenseSlotMap::insert_with_key() lets us create a connection with a key. Unfortanitely
        // the given function can not fail. Connection building can fail, so we have to return a
        // stub connection in that case (and then immediately remove it). A mess, I know.
        struct StubConnection;
        impl Connection for StubConnection {
            fn property_update(
                &self,
                _: EntityKey,
                _: &str,
                _: &Encodable,
            ) -> Result<(), Box<dyn Error>> {
                error!("property_changed() called on StubConnection");
                Err("StubConnection".into())
            }
            fn entity_destroyed(&self, _: &State, _: EntityKey) {
                error!("entity_destroyed() called on StubConnection");
            }
        }

        let mut failed_to_build = false;
        let request_tx = self.request_tx.clone();
        let root_entity = self.root_entity;
        let key = self.connections.insert_with_key(|key| {
            match ConnectionImpl::new_with_json(key, root_entity, builder, request_tx) {
                Ok(conn) => Box::new(conn),
                Err(e) => {
                    failed_to_build = true;
                    error!("error building connection: {}", e);
                    Box::new(StubConnection)
                }
            }
        });
        if failed_to_build {
            self.connections.remove(key);
        }
    }

    fn property_request(
        &self,
        handler: &mut dyn InboundMessageHandler,
        connection_key: ConnectionKey,
        entity: EntityKey,
        property: &str,
        action: PropertyRequest,
    ) -> Result<(), String> {
        match action {
            PropertyRequest::Set(value) => {
                handler.set(entity, property, &value)?;
            }
            PropertyRequest::Get => {
                let value = handler.get(entity, property)?;
                error!(
                    "get {:?}.{} returned {:?} (reply not implemented)",
                    entity, property, value
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

    fn request(&mut self, handler: &mut dyn InboundMessageHandler, request: Request) {
        match request.request {
            RequestType::Property((entity, prop), action) => {
                if let Err(e) =
                    self.property_request(handler, request.connection, entity, &prop, action)
                {
                    warn!("error processing request: {:?}", e);
                }
            }
            RequestType::Close => {
                info!("closing connection {:?}", request.connection);
                if self.connections.remove(request.connection).is_none() {
                    error!("invalid connection closed: {:?}", request.connection);
                }
            }
        };
    }
}

impl OutboundMessageHandler for ConnectionCollection {
    fn property_update(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(connection) = self.connections.get(connection) {
            connection.property_update(entity, property, &value)?;
            Ok(())
        } else {
            Err(format!("connection {:?} has died", connection).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug)]
    struct MockSession;

    impl Session for MockSession {
        fn yeet_bundle(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
            panic!("unecpected call");
        }

        fn max_packet_len(&self) -> usize {
            panic!("unecpected call");
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
                Err("session builder is supposed to error for test".into())
            }
        }
    }

    struct MockConnection(EntityKey);

    impl Connection for MockConnection {
        fn property_update(
            &self,
            _entity: EntityKey,
            _property: &str,
            _value: &Encodable,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn entity_destroyed(&self, _state: &crate::State, _entity: EntityKey) {}
    }

    struct MockInboundHandler(RefCell<Vec<(String, EntityKey, String)>>);

    impl MockInboundHandler {
        fn new() -> Self {
            Self(RefCell::new(Vec::new()))
        }
    }

    impl InboundMessageHandler for MockInboundHandler {
        fn set(
            &mut self,
            entity: EntityKey,
            property: &str,
            _value: &Decodable,
        ) -> Result<(), String> {
            self.0
                .borrow_mut()
                .push(("set".into(), entity, property.into()));
            Ok(())
        }
        fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String> {
            self.0
                .borrow_mut()
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
                .borrow_mut()
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
                .borrow_mut()
                .push(("unsubscribe".into(), entity, property.into()));
            Ok(())
        }
    }

    #[test]
    fn can_create_connection_from_session_builder() {
        let e = mock_keys(1);
        let mut cc = ConnectionCollection::new(e[0]);
        let builder = Box::new(MockSessionBuilder(true));
        cc.new_session_sender()
            .send(builder)
            .expect("failed to send connection builder");
        let mut handler = MockInboundHandler::new();
        assert_eq!(cc.connections.len(), 0);
        cc.process_inbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 1);
    }

    #[test]
    fn does_not_create_connection_when_building_session_fails() {
        let e = mock_keys(1);
        let mut cc = ConnectionCollection::new(e[0]);
        // False means building session will fail vvvvv
        let builder = Box::new(MockSessionBuilder(false));
        cc.new_session_sender()
            .send(builder)
            .expect("failed to send connection builder");
        let mut handler = MockInboundHandler::new();
        cc.process_inbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 0);
    }

    #[test]
    fn sends_requests_to_handler() {
        let e = mock_keys(1);
        let mut cc = ConnectionCollection::new(e[0]);
        let entities = mock_keys(2);
        let connections = mock_keys(1); // cc.connections.insert(Box::new(MockConnection(entities[0])));
        for request in vec![
            Request::new(
                connections[0],
                RequestType::Property((entities[0], "foo".into()), PropertyRequest::Subscribe),
            ),
            Request::new(
                connections[0],
                RequestType::Property((entities[1], "bar".into()), PropertyRequest::Get),
            ),
        ] {
            cc.request_tx.send(request).unwrap();
        }
        let mut handler = MockInboundHandler::new();
        cc.process_inbound_messages(&mut handler);
        assert_eq!(
            *handler.0.borrow(),
            vec![
                ("subscribe".into(), entities[0], "foo".into()),
                ("get".into(), entities[1], "bar".into()),
            ]
        );
    }
}
