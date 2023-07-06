use super::*;

new_key_type! {
    /// A handle to a client connection
    pub struct ConnectionKey;
}

const OUTGOING: &str = " <   ";
const INCOMING: &str = "   > ";

impl Display for ConnectionKey {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        format_slotmap_key(f, "Conn", *self)
    }
}

/// Manages a single client connection. Both the session type (TCP, WebRTC, etc) and the format
/// (JSON, etc) are abstracted.
pub trait Connection {
    /// Called at the start of the tick, process all inbound messages
    fn process_requests(&mut self, handler: &mut dyn RequestHandler);
    /// Send an event to the client, may not go through until flush()
    fn send_event(&self, handler: &dyn RequestHandler, event: Event);
    /// Called at the end of each network tick to send any pending bundles. If it returns
    fn flush(&mut self, handler: &mut dyn RequestHandler) -> Result<(), ()>;
    /// Called just after connection is removed from the connection map before it is dropped
    fn finalize(&mut self, handler: &dyn RequestHandler);
}

/// The main Connection implementation
pub struct ConnectionImpl {
    self_key: ConnectionKey,
    encoder: Box<dyn Encoder>,
    obj_map: Arc<dyn ObjectMap>,
    session: Mutex<Box<dyn Session>>,
    request_rx: Receiver<Request>,
    trace_level: TraceLevel,
    pending_get_requests: HashSet<(GenericId, String)>,
    should_close: AtomicBool,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        handler: &dyn RequestHandler,
        root_id: GenericId,
        session_builder: Box<dyn SessionBuilder>,
        trace_level: TraceLevel,
    ) -> Result<Self, Box<dyn Error>> {
        let obj_map = Arc::new(ObjectMapImpl::new(self_key));
        let root_obj_id = obj_map.get_or_create_object(handler, root_id);
        if root_obj_id.is_err() || root_obj_id.as_ref().unwrap() != &1 {
            // should never happen
            error!(
                "root ObjectID for {} is {:?} instead of Ok(1)",
                self_key, root_obj_id
            );
        }
        // TODO: let the client choose the format in the first message
        let (encoder, decoder) = json_protocol_impls();
        let (request_tx, request_rx) = channel();
        let handler = BundleHandler::new(self_key, decoder, obj_map.clone(), request_tx);
        let session = session_builder.build(Box::new(handler))?;
        if trace_level >= 2 {
            info!("created connection {} on {:?}", self_key, session);
        }
        Ok(Self {
            self_key,
            encoder,
            obj_map,
            session: Mutex::new(session),
            request_rx,
            trace_level,
            pending_get_requests: HashSet::new(),
            should_close: AtomicBool::new(false),
        })
    }

    fn process_request_method(
        &mut self,
        handler: &mut dyn RequestHandler,
        id: GenericId,
        property: &str,
        method: RequestMethod,
    ) -> RequestResult<()> {
        match method {
            RequestMethod::Action(value) => {
                handler.fire_action(self.self_key, id, property, value)?;
            }
            RequestMethod::Set(value) => {
                handler.set_property(self.self_key, id, property, value)?;
            }
            RequestMethod::Get => {
                // it doesn't matter if it's already there or not, it's not an error to make two
                // get requests but it will only result in one response.
                self.pending_get_requests.insert((id, property.into()));
            }
            RequestMethod::Subscribe => {
                // TODO: move this to object_map.rs
                self.obj_map.subscribe(handler, id, property)?;
                // If a signal is being subscribed to the get will fail, but that's fine
                self.pending_get_requests.insert((id, property.into()));
            }
            RequestMethod::Unsubscribe => {
                self.obj_map.unsubscribe(handler, id, property)?;
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
        if let Err(e) = session.send_data(&data) {
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
            let request = self.request_rx.try_recv();
            if self.trace_level >= 3 {
                match &request {
                    Ok(request) => info!("{}{}{:?}", self.self_key, INCOMING, request),
                    Err(TryRecvError::Disconnected) => info!("{} >DISCONNECTED", self.self_key),
                    _ => (),
                };
            }
            match request {
                Ok(Request::Method(entity, property, method)) => {
                    if let Err(e) =
                        self.process_request_method(handler, entity, &property, method.clone())
                    {
                        error!(
                            "failed to process {}{}{:?}.{} {:?}: {}",
                            self.self_key, INCOMING, entity, property, method, e
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

    fn send_event(&self, handler: &dyn RequestHandler, event: Event) {
        if matches!(event, Event::Method(_, _, EventMethod::Update, _)) {
            if self.trace_level >= 4 {
                info!("  {}{}{:?}", self.self_key, OUTGOING, event);
            }
        } else if self.trace_level >= 3 {
            info!("{}{}{:?}", self.self_key, OUTGOING, event);
        }

        let encode_ctx = new_encode_ctx(&*self.obj_map, handler);
        let buffer = match self.encoder.encode_event(&encode_ctx, &event) {
            Ok(buffer) => buffer,
            Err(e) => {
                error!("failed to encode {:?}: {}", event, e);
                self.should_close.store(true, SeqCst);
                return;
            }
        };
        self.queue_message(buffer);

        if let Event::Destroyed(entity) = event {
            self.obj_map.remove_entity(handler, entity);
        }
    }

    fn flush(&mut self, handler: &mut dyn RequestHandler) -> Result<(), ()> {
        let get_requests = std::mem::replace(&mut self.pending_get_requests, HashSet::new());
        for (entity, property) in get_requests.into_iter() {
            // When a client subscribes to a signal, we have no way of knowing it's a signal and
            // not a property, so it goes in the pending get requests list and is processed here.
            // That fails, and so we simply ignore errors here. There's probably a better way.
            if let Ok(value) = handler.get_property(self.self_key, entity, &property) {
                self.send_event(handler, Event::value(entity, property, value));
            }
        }
        if self.should_close.load(SeqCst) {
            Err(())
        } else {
            Ok(())
        }
    }

    fn finalize(&mut self, handler: &dyn RequestHandler) {
        let mut session = self.session.lock().unwrap();
        if self.trace_level >= 2 {
            info!("finalized connection {} on {:?}", self.self_key, session,);
        }
        session.close();
        self.obj_map.finalize(handler);
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
            obj_map: Arc::new(ObjectMapImpl::new(ConnectionKey::null())),
            session: Mutex::new(Box::new(session.clone())),
            request_rx,
            trace_level: 0,
            pending_get_requests: HashSet::new(),
            should_close: AtomicBool::new(false),
        };
        conn.obj_map
            .get_or_create_object(&MockRequestHandler::new(Ok(())), mock_generic_ids(1)[0])
            .unwrap();
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
        let e = mock_generic_ids(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(&handler, ev.clone());
        conn.flush(&mut handler).unwrap();
        // MockEncoder encodes the bundle using format!() as well, so this should pass as long as
        // everything's wired up correctly.
        sesh.assert_bundles_eq(vec![format!("{:?}", ev)]);
    }

    #[test]
    fn is_closed_when_encoding_fails() {
        let (mut conn, _, _tx) = setup(true, false);
        let e = mock_generic_ids(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(&handler, ev);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn is_closed_when_sending_fails() {
        let (mut conn, _, _tx) = setup(false, true);
        let e = mock_generic_ids(1);
        let ev = Event::signal(e[0], "foo".to_string(), 12.5.into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(&handler, ev);
        assert!(conn.flush(&mut handler).is_err());
    }

    #[test]
    fn does_not_keep_sending_events_after_sending_fails() {
        let (mut conn, sesh, _tx) = setup(false, true);
        let e = mock_generic_ids(2);
        let ev0 = Event::value(e[0], "foo".to_string(), 12.5.into());
        let ev1 = Event::update(e[1], "bar".to_string(), 8.into());
        let ev2 = Event::signal(e[0], "baz".to_string(), ().into());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.process_requests(&mut handler);
        conn.send_event(&handler, ev0.clone());
        conn.send_event(&handler, ev1);
        conn.send_event(&handler, ev2);
        assert!(conn.flush(&mut handler).is_err());
        // should only have the first request
        sesh.assert_bundles_eq(vec![format!("{:?}", ev0)]);
    }

    #[test]
    fn finalize_closes_session() {
        let (mut conn, session, _tx) = setup(false, true);
        assert!(!session.is_closed());
        let mut handler = MockRequestHandler::new(Ok(()));
        conn.finalize(&mut handler);
        assert!(session.is_closed());
    }
}

#[cfg(test)]
mod request_tests {
    use super::*;
    use test_common::*;

    #[test]
    fn action_request_makes_it_to_handler() {
        let (mut conn, _, tx) = setup(false, false);
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
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
        let e = mock_generic_ids(1);
        let mut handler =
            MockRequestHandler::new(Err(BadRequest("mock internal error".to_string())));
        let rq = Request::action(e[0], "act".to_string(), 7.into());
        tx.send(rq).unwrap();
        conn.process_requests(&mut handler);
        conn.flush(&mut handler).unwrap();
    }
}
