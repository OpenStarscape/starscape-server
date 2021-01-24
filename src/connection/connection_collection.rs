use super::*;

/// See try_to_build_connection for why this is needed
struct StubConnection;
impl Connection for StubConnection {
    fn process_requests(&mut self, _: &mut dyn InboundMessageHandler) {
        error!("StubConnection::process_requests() called");
    }
    fn send_event(&self, _: Event) {
        error!("StubConnection::send_event() called");
    }
    fn flush(&mut self, _: &mut dyn InboundMessageHandler) -> Result<(), ()> {
        error!("StubConnection::flush() called");
        Err(())
    }
    fn finalize(&mut self, _: &mut dyn InboundMessageHandler) {
        error!("StubConnection::finalize() called");
    }
}

struct NullInboundMessageHandler;
impl InboundMessageHandler for NullInboundMessageHandler {
    fn fire_action(
        &mut self,
        _: ConnectionKey,
        _: EntityKey,
        _: &str,
        _: Decoded,
    ) -> Result<(), String> {
        Err("connection failed".into())
    }
    fn set_property(
        &mut self,
        _: ConnectionKey,
        _: EntityKey,
        _: &str,
        _: Decoded,
    ) -> Result<(), String> {
        Err("connection failed".into())
    }
    fn get_property(&self, _: ConnectionKey, _: EntityKey, _: &str) -> Result<Encodable, String> {
        Err("connection failed".into())
    }
    fn subscribe(
        &mut self,
        _: ConnectionKey,
        _: EntityKey,
        _: &str,
    ) -> Result<Box<dyn Any>, String> {
        Err("connection failed".into())
    }
    fn unsubscribe(&mut self, _: Box<dyn Any>) -> Result<(), String> {
        Err("connection failed".into())
    }
}

/// Holds all the active connections for a game. process_requests() should be called by the game
/// once per network tick.
pub struct ConnectionCollection {
    root_entity: EntityKey,
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
    max_connections: usize,
    set_max_connections: bool,
}

impl ConnectionCollection {
    pub fn new(
        new_session_rx: Receiver<Box<dyn SessionBuilder>>,
        root_entity: EntityKey,
        max_connections: usize,
    ) -> Self {
        Self {
            root_entity,
            connections: DenseSlotMap::with_key(),
            new_session_rx,
            max_connections,
            set_max_connections: true,
        }
    }

    /// Handle incoming connection requests and messages from clients on the current thread. Should
    /// be called at the start of each network tick.
    pub fn process_inbound_messages(&mut self, handler: &mut dyn InboundMessageHandler) {
        // If we need to update the max connections property on the state, do so
        if self.set_max_connections {
            handler
                .set_property(
                    ConnectionKey::null(),
                    self.root_entity,
                    "max_conn_count",
                    Decoded::Integer(self.max_connections as i64),
                )
                .or_log_error("setting max connection count property");
            self.set_max_connections = false;
        }
        // Build sessions for any new clients that are trying to connect
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_to_build_connection(session_builder);
            handler
                .set_property(
                    ConnectionKey::null(),
                    self.root_entity,
                    "conn_count",
                    Decoded::Integer(self.connections.len() as i64),
                )
                .or_log_error("setting connection count property");
        }
        // Process requests on all connections
        for connection in self.connections.values_mut() {
            connection.process_requests(handler);
        }
    }

    /// Called after game state has been fully updated before waiting for the next tick
    pub fn flush_outbound_messages(&mut self, handler: &mut dyn InboundMessageHandler) {
        let failed_connections: Vec<ConnectionKey> = self
            .connections
            .iter_mut()
            .filter_map(|(key, connection)| match connection.flush(handler) {
                Ok(()) => None,
                Err(()) => Some(key),
            })
            .collect();
        for key in failed_connections {
            if let Some(mut connection) = self.connections.remove(key) {
                connection.finalize(handler);
            }
        }
    }

    fn try_to_build_connection(&mut self, builder: Box<dyn SessionBuilder>) {
        if self.connections.len() >= self.max_connections {
            error!(
                "maximum {} connections reached, new connection {:?} will not be added",
                self.connections.len(),
                builder
            );
            // Build a temporary connection in order to report the error to the client
            match ConnectionImpl::new_with_json(ConnectionKey::null(), self.root_entity, builder) {
                Ok(mut conn) => {
                    conn.send_event(Event::FatalError(format!(
                        "server full (max {} connections)",
                        self.max_connections
                    )));
                    conn.finalize(&mut NullInboundMessageHandler);
                }
                Err(e) => error!("failed to build connection: {}", e),
            };
            return;
        }

        // DenseSlotMap::insert_with_key() lets us create a connection with a key. Unfortanitely
        // the given function can not fail. Connection building can fail, so we have to return a
        // stub connection in that case (and then immediately remove it). A mess, I know.
        let mut failed_to_build = false;
        let root_entity = self.root_entity;
        let key = self.connections.insert_with_key(|key| {
            match ConnectionImpl::new_with_json(key, root_entity, builder) {
                Ok(conn) => Box::new(conn),
                Err(e) => {
                    failed_to_build = true;
                    error!("failed to build connection: {}", e);
                    Box::new(StubConnection)
                }
            }
        });
        if failed_to_build {
            self.connections.remove(key);
        }
    }

    pub fn finalize(&mut self, handler: &mut dyn InboundMessageHandler) {
        for (_, mut connection) in self.connections.drain() {
            connection.send_event(Event::FatalError("server has shut down".to_string()));
            let _ = connection.flush(handler);
            connection.finalize(handler);
        }
    }
}

impl OutboundMessageHandler for ConnectionCollection {
    fn event(&self, connection: ConnectionKey, event: Event) {
        if let Some(connection) = self.connections.get(connection) {
            connection.send_event(event);
        } else {
            error!(
                "{:?} does not exist, could not send {:?}",
                connection, event
            );
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
            Ok(())
        }

        fn max_packet_len(&self) -> usize {
            panic!("unecpected call");
        }

        fn close(&mut self) {
            panic!("unecpected call");
        }
    }

    /// Contained value is if building session should succeed
    #[derive(Debug)]
    struct MockSessionBuilder(bool);

    impl SessionBuilder for MockSessionBuilder {
        fn build(
            self: Box<Self>,
            _handler: Box<dyn InboundBundleHandler>,
        ) -> Result<Box<dyn Session>, Box<dyn Error>> {
            if self.0 {
                Ok(Box::new(MockSession))
            } else {
                Err("session builder is supposed to error for test".into())
            }
        }
    }

    struct MockConnection {
        flush_succeeds: bool,
    }

    impl Connection for MockConnection {
        fn process_requests(&mut self, _: &mut dyn InboundMessageHandler) {}
        fn send_event(&self, _: Event) {}
        fn flush(&mut self, _: &mut dyn InboundMessageHandler) -> Result<(), ()> {
            if self.flush_succeeds {
                Ok(())
            } else {
                Err(())
            }
        }
        fn finalize(&mut self, _: &mut dyn InboundMessageHandler) {}
    }

    struct MockInboundHandler(RefCell<Vec<(String, EntityKey, String)>>);

    impl MockInboundHandler {
        fn new() -> Self {
            Self(RefCell::new(Vec::new()))
        }
    }

    impl InboundMessageHandler for MockInboundHandler {
        fn fire_action(
            &mut self,
            _: ConnectionKey,
            entity: EntityKey,
            name: &str,
            _: Decoded,
        ) -> Result<(), String> {
            self.0
                .borrow_mut()
                .push(("fire".into(), entity, name.into()));
            Ok(())
        }
        fn set_property(
            &mut self,
            _: ConnectionKey,
            entity: EntityKey,
            name: &str,
            _: Decoded,
        ) -> Result<(), String> {
            self.0
                .borrow_mut()
                .push(("set".into(), entity, name.into()));
            Ok(())
        }
        fn get_property(
            &self,
            _: ConnectionKey,
            entity: EntityKey,
            name: &str,
        ) -> Result<Encodable, String> {
            self.0
                .borrow_mut()
                .push(("get".into(), entity, name.into()));
            Ok(Encodable::Null)
        }
        fn subscribe(
            &mut self,
            _: ConnectionKey,
            entity: EntityKey,
            name: &str,
        ) -> Result<Box<dyn Any>, String> {
            self.0
                .borrow_mut()
                .push(("subscribe".into(), entity, name.into()));
            let subscription: (EntityKey, String) = (entity, name.into());
            Ok(Box::new(subscription))
        }
        fn unsubscribe(&mut self, subscription: Box<dyn Any>) -> Result<(), String> {
            let subscription: Box<(EntityKey, String)> =
                subscription.downcast().expect("subscription of wrong type");
            self.0
                .borrow_mut()
                .push(("unsubscribe".into(), subscription.0, subscription.1));
            Ok(())
        }
    }

    #[test]
    fn can_create_connection_from_session_builder() {
        let e = mock_keys(1);
        let (session_tx, session_rx) = channel();
        let mut cc = ConnectionCollection::new(session_rx, e[0], usize::MAX);
        let builder = Box::new(MockSessionBuilder(true));
        session_tx
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
        let (session_tx, session_rx) = channel();
        let mut cc = ConnectionCollection::new(session_rx, e[0], usize::MAX);
        // False means building session will fail vvvvv
        let builder = Box::new(MockSessionBuilder(false));
        session_tx
            .send(builder)
            .expect("failed to send connection builder");
        let mut handler = MockInboundHandler::new();
        cc.process_inbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 0);
    }

    #[test]
    fn building_connections_fail_after_max_connections_reached() {
        let e = mock_keys(1);
        let (session_tx, session_rx) = channel();
        let mut cc = ConnectionCollection::new(session_rx, e[0], 2);
        session_tx
            .send(Box::new(MockSessionBuilder(true)))
            .expect("failed to send connection builder");
        session_tx
            .send(Box::new(MockSessionBuilder(true)))
            .expect("failed to send connection builder");
        session_tx
            .send(Box::new(MockSessionBuilder(true)))
            .expect("failed to send connection builder");
        let mut handler = MockInboundHandler::new();
        cc.process_inbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 2);
        session_tx
            .send(Box::new(MockSessionBuilder(true)))
            .expect("failed to send connection builder");
        cc.process_inbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 2);
    }

    #[test]
    fn does_not_remove_connections_that_succeed_to_flush() {
        let e = mock_keys(1);
        let (_, session_rx) = channel();
        let mut cc = ConnectionCollection::new(session_rx, e[0], usize::MAX);
        cc.connections.insert(Box::new(MockConnection {
            flush_succeeds: true,
        }));
        assert_eq!(cc.connections.len(), 1);
        let mut handler = MockInboundHandler::new();
        cc.flush_outbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 1);
    }

    #[test]
    fn removes_connections_that_fail_to_flush() {
        let e = mock_keys(1);
        let (_, session_rx) = channel();
        let mut cc = ConnectionCollection::new(session_rx, e[0], usize::MAX);
        cc.connections.insert(Box::new(MockConnection {
            flush_succeeds: false,
        }));
        assert_eq!(cc.connections.len(), 1);
        let mut handler = MockInboundHandler::new();
        cc.flush_outbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 0);
    }

    /*
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
        cc.flush_outbound_messages(&mut handler);
        assert_eq!(
            *handler.0.borrow(),
            vec![
                ("subscribe".into(), entities[0], "foo".into()),
                ("get".into(), entities[1], "bar".into()),
            ]
        );
    }
    */

    // TODO: test connections are finalized
}
