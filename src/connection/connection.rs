use super::*;

new_key_type! {
    /// A handle to a client connection
    pub struct ConnectionKey;
}

/// Manages a single client connection. Both the session type (TCP, WebRTC, etc) and the format
/// (JSON, etc) are abstracted.
pub trait Connection {
    /// Called at the start of the tick, process all inbound messages
    fn process_requests(&mut self, handler: &mut dyn RequestHandler);
    /// Send an event to the client, may not go through until flush()
    fn send_event(&self, event: Event);
    /// Called at the end of each network tick to send any pending bundles. If it returns
    fn flush(&mut self, handler: &mut dyn RequestHandler) -> Result<(), ()>;
    /// Called just after connection is removed from the connection map before it is dropped
    fn finalize(&mut self, handler: &mut dyn RequestHandler);
}

/// The main Connection implementation
pub struct ConnectionImpl {
    self_key: ConnectionKey,
    encoder: Box<dyn Encoder>,
    obj_map: Arc<dyn ObjectMap>,
    session: Mutex<Box<dyn Session>>,
    request_rx: Receiver<Request>,
    pending_get_requests: HashSet<(EntityKey, String)>,
    subscriptions: HashMap<(EntityKey, String), Box<dyn Any>>,
    should_close: AtomicBool,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        root_entity: EntityKey,
        session_builder: Box<dyn SessionBuilder>,
    ) -> Result<Self, Box<dyn Error>> {
        let obj_map = Arc::new(ObjectMapImpl::new());
        let root_obj_id = obj_map.get_or_create_object(root_entity);
        if root_obj_id != 1 {
            // should never happen
            error!(
                "root ObjectID for {:?} is {} instead of 1",
                self_key, root_obj_id
            );
        }
        // TODO: let the client choose the format in the first message
        let (encoder, decoder) = json_protocol_impls();
        let (request_tx, request_rx) = channel();
        let handler = BundleHandler::new(self_key, decoder, obj_map.clone(), request_tx);
        let session = session_builder.build(Box::new(handler))?;
        info!("created connection {:?} on {:?}", self_key, session);
        Ok(Self {
            self_key,
            encoder,
            obj_map,
            session: Mutex::new(session),
            request_rx,
            pending_get_requests: HashSet::new(),
            subscriptions: HashMap::new(),
            should_close: AtomicBool::new(false),
        })
    }

    fn process_request_method(
        &mut self,
        handler: &mut dyn RequestHandler,
        entity: EntityKey,
        property: &str,
        method: RequestMethod,
    ) -> RequestResult<()> {
        use std::collections::hash_map::Entry;
        match method {
            RequestMethod::Action(value) => {
                handler.fire_action(self.self_key, entity, property, value)?;
            }
            RequestMethod::Set(value) => {
                handler.set_property(self.self_key, entity, property, value)?;
            }
            RequestMethod::Get => {
                // it doesn't matter if it's already there or not, it's not an error to make two
                // get requests but it will only result in one response.
                self.pending_get_requests.insert((entity, property.into()));
            }
            RequestMethod::Subscribe => {
                match self.subscriptions.entry((entity, property.to_string())) {
                    Entry::Occupied(_) => {
                        return Err(BadRequest("tried to subscribe multiple times".into()))
                    }
                    Entry::Vacant(entry) => {
                        let sub = handler.subscribe(self.self_key, entity, property)?;
                        entry.insert(sub);
                        self.pending_get_requests.insert((entity, property.into()));
                    }
                }
            }
            RequestMethod::Unsubscribe => {
                let key = (entity, property.to_string());
                match self.subscriptions.remove(&key) {
                    Some(entry) => handler.unsubscribe(entry)?,
                    None => {
                        return Err(BadRequest(
                            "tried to unsubscribe when not subscribed".into(),
                        ))
                    }
                }
            }
        };
        Ok(())
    }

    fn queue_message(&self, data: Vec<u8>) {
        // Drop data if we are closing. This looks not threadsafe and def needs a refactor but the
        // worst that can happen is the session logs a warning and ignores so who cares.
        if self.should_close.load(SeqCst) {
            return;
        }
        let mut session = self.session.lock().unwrap();
        if let Err(e) = session.yeet_bundle(&data) {
            warn!("closing session due to problem sending bundle: {}", e);
            self.should_close.store(true, SeqCst);
            session.close();
        }
    }
}

impl Connection for ConnectionImpl {
    fn process_requests(&mut self, handler: &mut dyn RequestHandler) {
        use std::sync::mpsc::TryRecvError;
        loop {
            match self.request_rx.try_recv() {
                Ok(Request::Method(entity, property, method)) => {
                    if let Err(e) =
                        self.process_request_method(handler, entity, &property, method.clone())
                    {
                        error!(
                            "failed to process {:?} on {:?}::{:?}.{}: {}",
                            method, self.self_key, entity, property, e
                        );
                        // TODO: send error to client
                    }
                }
                Ok(Request::Close) | Err(TryRecvError::Disconnected) => {
                    self.should_close.store(true, SeqCst);
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    fn send_event(&self, event: Event) {
        let buffer = match self
            .encoder
            .encode_event(self.obj_map.as_encode_ctx(), &event)
        {
            Ok(buffer) => buffer,
            Err(e) => {
                error!("failed to encode {:?}: {}", event, e);
                self.should_close.store(true, SeqCst);
                return;
            }
        };
        self.queue_message(buffer);

        if let Event::Destroyed(entity) = event {
            self.obj_map.remove_entity(entity);
        }
    }

    fn flush(&mut self, handler: &mut dyn RequestHandler) -> Result<(), ()> {
        let get_requests = std::mem::replace(&mut self.pending_get_requests, HashSet::new());
        for (entity, property) in get_requests.into_iter() {
            // When a client subscribes to a signal, we have no way of knowing it's a signal and
            // not a property, so it goes in the pending get requests list and is processed here.
            // That fails, and so we simply ignore errors here. There's probably a better way.
            if let Ok(value) = handler.get_property(self.self_key, entity, &property) {
                self.send_event(Event::value(entity, property, value));
            }
        }
        if self.should_close.load(SeqCst) {
            Err(())
        } else {
            Ok(())
        }
    }

    fn finalize(&mut self, handler: &mut dyn RequestHandler) {
        info!(
            "finalized connection {:?} on {:?}",
            self.self_key,
            self.session.lock().unwrap()
        );
        for ((entity, prop), subscription) in self.subscriptions.drain() {
            if let Err(e) = handler.unsubscribe(subscription) {
                warn!(
                    "failed to unsubscribe from {:?}.{} during finalization of {:?}: {}",
                    entity, prop, self.self_key, e
                );
            }
        }
    }
}

#[cfg(test)]
mod test_common {
    use super::*;

    pub struct MockEncoder {
        should_error: bool,
    }

    impl MockEncoder {
        pub fn new(should_error: bool) -> Self {
            Self { should_error }
        }
    }

    impl Encoder for MockEncoder {
        /// Encode an event
        fn encode_event(
            &self,
            _: &dyn EncodeCtx,
            event: &Event,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            if self.should_error {
                Err("MockEncoder error".into())
            } else {
                Ok(format!("{:?}", event).as_bytes().into())
            }
        }
    }

    pub struct MockObjectMap;

    impl ObjectMap for MockObjectMap {
        fn get_object(&self, _: EntityKey) -> Option<ObjectId> {
            panic!("unexpected call");
        }

        fn get_or_create_object(&self, _: EntityKey) -> ObjectId {
            panic!("unexpected call");
        }

        fn get_entity(&self, _: ObjectId) -> Option<EntityKey> {
            panic!("unexpected call");
        }

        fn remove_entity(&self, _: EntityKey) -> Option<ObjectId> {
            panic!("unexpected call");
        }

        fn as_encode_ctx(&self) -> &dyn EncodeCtx {
            self
        }

        fn as_decode_ctx(&self) -> &dyn DecodeCtx {
            self
        }
    }

    pub fn setup(
        encoder_error: bool,
        session_error: bool,
    ) -> (ConnectionImpl, MockSession, Sender<Request>) {
        let encoder = MockEncoder::new(encoder_error);
        let session = MockSession::new(session_error);
        let (request_tx, request_rx) = channel();
        let conn = ConnectionImpl {
            self_key: ConnectionKey::null(),
            encoder: Box::new(encoder),
            obj_map: Arc::new(MockObjectMap),
            session: Mutex::new(Box::new(session.clone())),
            request_rx,
            pending_get_requests: HashSet::new(),
            subscriptions: HashMap::new(),
            should_close: AtomicBool::new(false),
        };
        (conn, session, request_tx)
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;
    use test_common::*;

    #[test]
    fn sends_signal_event() {
        let (mut conn, sesh, _tx) = setup(false, false);
        let e = mock_keys(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(ev.clone());
        conn.flush(&mut handler).unwrap();
        // MockEncoder encodes the bundle using format!() as well, so this should pass as long as
        // everything's wired up correctly.
        sesh.assert_bundles_eq(vec![format!("{:?}", ev)]);
    }

    #[test]
    fn is_closed_when_encoding_fails() {
        let (mut conn, _, _tx) = setup(true, false);
        let e = mock_keys(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(ev);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn is_closed_when_sending_fails() {
        let (mut conn, _, _tx) = setup(false, true);
        let e = mock_keys(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(ev);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn does_not_keep_sending_events_after_sending_fails() {
        let (mut conn, sesh, _tx) = setup(false, true);
        let e = mock_keys(2);
        let ev0 = Event::value(e[0], "foo".to_string(), 12.5.into());
        let ev1 = Event::update(e[1], "bar".to_string(), 8.into());
        let ev2 = Event::signal(e[0], "baz".to_string(), ().into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(ev0.clone());
        conn.send_event(ev1);
        conn.send_event(ev2);
        assert!(conn.flush(&mut handler).is_err());
        // should only have the first request
        sesh.assert_bundles_eq(vec![format!("{:?}", ev0)]);
    }
}

#[cfg(test)]
mod request_tests {
    use super::*;
    use test_common::*;

    #[test]
    fn action_request_makes_it_to_handler() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let rq = Request::action(e[0], "act".to_string(), 7.into());
        tx.send(rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![rq]);
    }

    #[test]
    fn sub_request_results_in_get() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let sub_rq = Request::subscribe(e[0], "prop".to_string());
        tx.send(sub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![sub_rq, Request::get(e[0], "prop".to_string())]);
    }

    #[test]
    fn get_request_works() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let rq = Request::get(e[0], "prop".to_string());
        tx.send(rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![rq]);
    }

    #[test]
    fn does_not_sub_multiple_times_in_one_tick() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let sub_rq = Request::subscribe(e[0], "prop".to_string());
        tx.send(sub_rq.clone()).unwrap();
        tx.send(sub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![sub_rq, Request::get(e[0], "prop".to_string())]);
    }

    #[test]
    fn does_not_sub_multiple_times_in_multiple_ticks() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let sub_rq = Request::subscribe(e[0], "prop".to_string());
        tx.send(sub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        tx.send(sub_rq.clone()).unwrap();
        tx.send(sub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![sub_rq, Request::get(e[0], "prop".to_string())]);
    }

    #[test]
    fn sub_unsub_in_one_tick_works() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let sub_rq = Request::subscribe(e[0], "prop".to_string());
        let unsub_rq = Request::unsubscribe(e[0], "prop".to_string());
        tx.send(sub_rq.clone()).unwrap();
        tx.send(unsub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![
            sub_rq,
            unsub_rq,
            Request::get(e[0], "prop".to_string()),
        ]);
    }

    #[test]
    fn sub_unsub_in_multiple_ticks_works() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler = MockRequestHandler::new(Ok(()));
        let sub_rq = Request::subscribe(e[0], "prop".to_string());
        let unsub_rq = Request::unsubscribe(e[0], "prop".to_string());
        tx.send(sub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        tx.send(unsub_rq.clone()).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
        handler.assert_requests_eq(vec![
            sub_rq,
            Request::get(e[0], "prop".to_string()),
            unsub_rq,
        ]);
    }

    #[test]
    fn close_request_results_in_flush_returning_err() {
        let (mut conn, _, tx) = setup(false, false);
        let mut handler = MockRequestHandler::new(Ok(()));
        tx.send(Request::Close).unwrap();
        conn.process_requests(&mut handler);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn closed_when_request_tx_dropped() {
        let (mut conn, _sesh, _) = setup(false, false);
        //                    ^ tx is dropped here
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn not_closed_on_request_internal_error() {
        let (mut conn, _sesh, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler =
            MockRequestHandler::new(Err(InternalError("mock internal error".to_string())));
        let rq = Request::action(e[0], "act".to_string(), 7.into());
        tx.send(rq).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
    }

    #[test]
    fn not_closed_on_bad_request_error() {
        let (mut conn, _sesh, tx) = setup(false, false);
        let e = mock_keys(1);
        let mut handler =
            MockRequestHandler::new(Err(BadRequest("mock internal error".to_string())));
        let rq = Request::action(e[0], "act".to_string(), 7.into());
        tx.send(rq).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
    }
}
