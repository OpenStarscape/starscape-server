use super::*;

/// See try_to_build_connection for why this is needed
struct StubConnection;
impl Connection for StubConnection {
    fn process_requests(&mut self, _: &mut dyn RequestHandler) {
        error!("StubConnection::process_requests() called");
    }
    fn send_event(&self, _: &dyn RequestHandler, _: Event) {
        error!("StubConnection::send_event() called");
    }
    fn flush(&mut self, _: &mut dyn RequestHandler) -> Result<(), ()> {
        error!("StubConnection::flush() called");
        Err(())
    }
    fn finalize(&mut self, _: &dyn RequestHandler) {
        error!("StubConnection::finalize() called");
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
    pub fn process_inbound_messages(&mut self, handler: &mut dyn RequestHandler) {
        // If we need to update the max connections property on the state, do so
        if self.set_max_connections {
            handler
                .set_property(
                    ConnectionKey::null(),
                    self.root_entity,
                    "max_conn_count",
                    Value::Integer(self.max_connections as i64),
                )
                .or_log_error("setting max connection count property");
            self.set_max_connections = false;
        }
        // Build sessions for any new clients that are trying to connect
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_to_build_connection(handler, session_builder);
            handler
                .set_property(
                    ConnectionKey::null(),
                    self.root_entity,
                    "conn_count",
                    Value::Integer(self.connections.len() as i64),
                )
                .or_log_error("setting connection count property");
        }
        // Process requests on all connections
        for connection in self.connections.values_mut() {
            connection.process_requests(handler);
        }
    }

    /// Called after game state has been fully updated before waiting for the next tick
    pub fn flush_outbound_messages(&mut self, handler: &mut dyn RequestHandler) {
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

    fn try_to_build_connection(
        &mut self,
        handler: &dyn RequestHandler,
        builder: Box<dyn SessionBuilder>,
    ) {
        if self.connections.len() >= self.max_connections {
            error!(
                "maximum {} connections reached, new connection {:?} will not be added",
                self.connections.len(),
                builder
            );
            // Build a temporary connection in order to report the error to the client
            match ConnectionImpl::new(ConnectionKey::null(), handler, self.root_entity, builder) {
                Ok(mut conn) => {
                    conn.send_event(
                        handler,
                        Event::FatalError(format!(
                            "server full (max {} connections)",
                            self.max_connections
                        )),
                    );
                    conn.finalize(handler);
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
            match ConnectionImpl::new(key, handler, root_entity, builder) {
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

    pub fn finalize(&mut self, handler: &mut dyn RequestHandler) {
        for (_, mut connection) in self.connections.drain() {
            connection.send_event(
                handler,
                Event::FatalError("server has shut down".to_string()),
            );
            let _ = connection.flush(handler);
            connection.finalize(handler);
        }
    }
}

impl EventHandler for ConnectionCollection {
    fn event(&self, handler: &dyn RequestHandler, connection: ConnectionKey, event: Event) {
        if let Some(connection) = self.connections.get(connection) {
            connection.send_event(handler, event);
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

    #[derive(Debug)]
    struct MockSession;

    impl Session for MockSession {
        fn send_data(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn max_packet_len(&self) -> usize {
            usize::MAX
        }

        fn close(&mut self) {}
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
        fn process_requests(&mut self, _: &mut dyn RequestHandler) {}
        fn send_event(&self, _: &dyn RequestHandler, _: Event) {}
        fn flush(&mut self, _: &mut dyn RequestHandler) -> Result<(), ()> {
            if self.flush_succeeds {
                Ok(())
            } else {
                Err(())
            }
        }
        fn finalize(&mut self, _: &dyn RequestHandler) {}
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
        let mut handler = MockRequestHandler::new(Ok(()));
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
        let mut handler = MockRequestHandler::new(Ok(()));
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
        let mut handler = MockRequestHandler::new(Ok(()));
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
        let mut handler = MockRequestHandler::new(Ok(()));
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
        let mut handler = MockRequestHandler::new(Ok(()));
        cc.flush_outbound_messages(&mut handler);
        assert_eq!(cc.connections.len(), 0);
    }

    // TODO: test connections are finalized
}
