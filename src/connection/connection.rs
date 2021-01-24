use super::*;

new_key_type! {
    /// A handle to a client connection
    pub struct ConnectionKey;
}

/// Manages a single client connection. Both the session type (TCP, WebRTC, etc) and the format
/// (JSON, etc) are abstracted.
pub trait Connection {
    /// Called at the start of the tick, process all inbound messages
    fn process_requests(&mut self, handler: &mut dyn InboundMessageHandler);
    /// Send an event to the client, may not go through until flush()
    fn send_event(&self, event: Event);
    /// Called at the end of each network tick to send any pending bundles. If it returns
    fn flush(&mut self, handler: &mut dyn InboundMessageHandler) -> Result<(), ()>;
    /// Called just after connection is removed from the connection map before it is dropped
    fn finalize(&mut self, handler: &mut dyn InboundMessageHandler);
}

/// Receives data from the session layer (on the session's thread), decodes it into requests and
/// sends those off to be processed by the session on the main thead.
struct ConnectionInboundHandler {
    connection_key: ConnectionKey,
    decoder: Box<dyn Decoder>,
    decode_ctx: Arc<dyn DecodeCtx>,
    request_tx: Sender<Request>,
}

impl InboundBundleHandler for ConnectionInboundHandler {
    fn handle(&mut self, data: &[u8]) {
        match self
            .decoder
            .decode(self.decode_ctx.as_ref(), data.to_owned())
        {
            Ok(requests) => {
                requests.into_iter().for_each(|request| {
                    if let Err(e) = self.request_tx.send(request) {
                        warn!("failed to handle data for {:?}: {}", self.connection_key, e);
                    }
                });
            }
            Err(e) => {
                warn!(
                    "can't decode inbound bundle: {} on {:?}",
                    e, self.connection_key
                );
            }
        }
    }

    fn close(&mut self) {
        if let Err(e) = self.request_tx.send(Request::Close) {
            warn!("failed to close {:?}: {}", self.connection_key, e);
        }
    }
}

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
        encoder: Box<dyn Encoder>,
        decoder: Box<dyn Decoder>,
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
        let (request_tx, request_rx) = channel();
        let handler = ConnectionInboundHandler {
            connection_key: self_key,
            decoder,
            decode_ctx: obj_map.clone(),
            request_tx,
        };
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

    pub fn new_with_json(
        self_key: ConnectionKey,
        root_entity: EntityKey,
        session_builder: Box<dyn SessionBuilder>,
    ) -> Result<Self, Box<dyn Error>> {
        let (encoder, decoder) = json_protocol_impls();
        Self::new(self_key, root_entity, session_builder, encoder, decoder)
    }

    fn process_request_method(
        &mut self,
        handler: &mut dyn InboundMessageHandler,
        entity: EntityKey,
        property: &str,
        method: RequestMethod,
    ) -> Result<(), String> {
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
                    Entry::Occupied(_) => return Err("tried to subscribe multiple times".into()),
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
                    None => return Err("tried to unsubscribe when not subscribed".into()),
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
    fn process_requests(&mut self, handler: &mut dyn InboundMessageHandler) {
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

    fn flush(&mut self, handler: &mut dyn InboundMessageHandler) -> Result<(), ()> {
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

    fn finalize(&mut self, handler: &mut dyn InboundMessageHandler) {
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

/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use Encodable::*;

    struct MockEncoder {
        log: Vec<(ObjectId, String, Encodable)>,
    }

    impl MockEncoder {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self { log: Vec::new() }))
        }
    }

    impl Encoder for Rc<RefCell<MockEncoder>> {
        fn encode_property_update(
            &self,
            object: ObjectId,
            property: &str,
            _ctx: &dyn EncodeCtx,
            value: &Encodable,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            self.borrow_mut()
                .log
                .push((object, property.to_owned(), (*value).clone()));
            Ok(vec![])
        }
        fn encode_get_response(
            &self,
            object: ObjectId,
            property: &str,
            _ctx: &dyn EncodeCtx,
            value: &Encodable,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            self.borrow_mut()
                .log
                .push((object, property.to_owned(), (*value).clone()));
            Ok(vec![])
        }
        fn encode_signal(
            &self,
            object: ObjectId,
            property: &str,
            _ctx: &dyn EncodeCtx,
            value: &Encodable,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            self.borrow_mut()
                .log
                .push((object, property.to_owned(), (*value).clone()));
            Ok(vec![])
        }
        fn encode_error(&self, _: &str) -> Result<Vec<u8>, Box<dyn Error>> {
            Ok(vec![])
        }
    }

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
            panic!("unexpected call");
        }
    }

    struct MockObjectMap(EntityKey, ObjectId);

    impl ObjectMap for MockObjectMap {
        fn get_object(&self, entity: EntityKey) -> Option<ObjectId> {
            if self.0 == entity {
                Some(self.1)
            } else {
                None
            }
        }

        fn get_or_create_object(&self, _entity: EntityKey) -> ObjectId {
            panic!("unexpected call");
        }

        fn get_entity(&self, object: ObjectId) -> Option<EntityKey> {
            if self.1 == object {
                Some(self.0)
            } else {
                None
            }
        }

        fn remove_entity(&self, _entity: EntityKey) -> Option<ObjectId> {
            panic!("unexpected call");
        }

        fn as_encode_ctx(&self) -> &dyn EncodeCtx {
            self
        }

        fn as_decode_ctx(&self) -> &dyn DecodeCtx {
            self
        }
    }

    struct Test {
        encoder: Rc<RefCell<MockEncoder>>,
        conn: ConnectionImpl,
        entity: EntityKey,
        obj_id: ObjectId,
    }

    impl Test {
        fn new() -> Self {
            let encoder = MockEncoder::new();
            let entities = mock_keys(1);
            let entity = entities[0];
            let obj_id = 1;
            let (_, request_rx) = channel();
            let conn = ConnectionImpl {
                self_key: ConnectionKey::null(),
                encoder: Box::new(encoder.clone()),
                obj_map: Arc::new(MockObjectMap(entity, obj_id)),
                session: Mutex::new(Box::new(MockSession)),
                request_rx,
                pending_get_requests: HashSet::new(),
                subscriptions: HashMap::new(),
                should_close: AtomicBool::new(false),
            };
            Self {
                encoder,
                conn,
                entity,
                obj_id,
            }
        }
    }

    #[test]
    fn serializes_normal_property_update() {
        let test = Test::new();
        test.conn
            .property_value(test.entity, "foo", &Scalar(12.5), true);
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Scalar(12.5))]
        );
    }
}
*/
