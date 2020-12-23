use super::*;

new_key_type! {
    /// A handle to a client connection
    pub struct ConnectionKey;
}

/// Manages a single client connection. Both the session type (TCP, WebRTC, etc) and the format
/// (JSON, etc) are abstracted.
pub trait Connection {
    /// Send a property's value to a client. If is_update is true this is a response to a change in
    /// a subscribed property. If false, this is a response to a get request.
    fn property_value(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
        is_update: bool,
    ) -> Result<(), Box<dyn Error>>;
    /// Inform a client that an entity no longer exists on the server.
    fn entity_destroyed(&self, state: &State, entity: EntityKey);
    /// Process a request from the client
    fn handle_request(
        &mut self,
        handler: &mut dyn InboundMessageHandler,
        entity: EntityKey,
        property: &str,
        action: PropertyRequest,
    );
    /// Called at the end of each network tick to send any pending bundles
    fn flush(&mut self, handler: &mut dyn InboundMessageHandler);
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
                    if let Err(e) = self
                        .request_tx
                        .send(Request::new(self.connection_key, request))
                    {
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
        if let Err(e) = self
            .request_tx
            .send(Request::new(self.connection_key, RequestType::Close))
        {
            warn!("failed to close {:?}: {}", self.connection_key, e);
        }
    }
}

pub struct ConnectionImpl {
    self_key: ConnectionKey,
    encoder: Box<dyn Encoder>,
    obj_map: Arc<dyn ObjectMap>,
    session: Mutex<Box<dyn Session>>,
    pending_get_requests: HashSet<(EntityKey, String)>,
    subscriptions: HashSet<(EntityKey, String)>,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        root_entity: EntityKey,
        session_builder: Box<dyn SessionBuilder>,
        request_tx: Sender<Request>,
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
        let handler = ConnectionInboundHandler {
            connection_key: self_key,
            decoder,
            decode_ctx: obj_map.clone(),
            request_tx,
        };
        let session = session_builder.build(Box::new(handler))?;
        Ok(Self {
            self_key,
            encoder,
            obj_map,
            session: Mutex::new(session),
            pending_get_requests: HashSet::new(),
            subscriptions: HashSet::new(),
        })
    }

    pub fn new_with_json(
        self_key: ConnectionKey,
        root_entity: EntityKey,
        session_builder: Box<dyn SessionBuilder>,
        request_tx: Sender<Request>,
    ) -> Result<Self, Box<dyn Error>> {
        let (encoder, decoder) = json_protocol_impls();
        Self::new(
            self_key,
            root_entity,
            session_builder,
            request_tx,
            encoder,
            decoder,
        )
    }

    fn queue_message(&self, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut session = self
            .session
            .lock()
            .map_err(|e| format!("failed to lock session: {}", e))?;
        session
            .yeet_bundle(&data)
            .map_err(|e| format!("failed to yeet bundle: {}", e))?;
        Ok(())
    }
}

impl Connection for ConnectionImpl {
    fn property_value(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
        is_update: bool,
    ) -> Result<(), Box<dyn Error>> {
        let object = self
            .obj_map
            .get_object(entity)
            .ok_or_else(|| format!("{:?} not in object map", entity))?;
        trace!("{:?}::{}.{} = {:?}", self.self_key, object, property, value);
        let buffer = if is_update {
            self.encoder.encode_property_update(
                object,
                property,
                self.obj_map.as_encode_ctx(),
                value,
            )?
        } else {
            self.encoder.encode_get_response(
                object,
                property,
                self.obj_map.as_encode_ctx(),
                value,
            )?
        };
        self.queue_message(buffer)?;
        Ok(())
    }

    fn entity_destroyed(&self, _state: &State, entity: EntityKey) {
        self.obj_map.remove_entity(entity);
        // TODO: tell client object was destroyed
    }

    fn handle_request(
        &mut self,
        handler: &mut dyn InboundMessageHandler,
        entity: EntityKey,
        property: &str,
        action: PropertyRequest,
    ) {
        if let Err(e) = match action {
            PropertyRequest::Set(value) => handler.set(entity, property, value),
            PropertyRequest::Get => {
                // it doesn't matter if it's already there or not, it's not an error to make two
                // get requests but it will only result in one response.
                self.pending_get_requests.insert((entity, property.into()));
                Ok(())
            }
            PropertyRequest::Subscribe => {
                let key = (entity, property.to_string());
                if self.subscriptions.contains(&key) {
                    Err("tried to subscribe multiple times".into())
                } else {
                    handler
                        .subscribe(entity, property, self.self_key)
                        .map(|()| {
                            self.subscriptions.insert(key);
                        })
                }
            }
            PropertyRequest::Unsubscribe => {
                let key = (entity, property.to_string());
                if self.subscriptions.contains(&key) {
                    handler
                        .unsubscribe(entity, property, self.self_key)
                        .map(|()| {
                            self.subscriptions.remove(&key);
                        })
                } else {
                    Err("tried to unsubscribe when not subscribed".into())
                }
            }
        } {
            warn!(
                "failed to process message to {:?}::{:?}.{}: {}",
                self.self_key, entity, property, e
            );
        }
    }

    fn flush(&mut self, handler: &mut dyn InboundMessageHandler) {
        let pending_get_requests =
            std::mem::replace(&mut self.pending_get_requests, HashSet::new());
        for (entity, property) in pending_get_requests.iter() {
            match handler.get(*entity, property) {
                Ok(value) => {
                    if let Err(e) = self.property_value(*entity, property, &value, false) {
                        warn!("failed to get {:?}.{}: {}", entity, property, e);
                    }
                }
                Err(e) => warn!("failed to sand value on {:?}: {}", self.self_key, e),
            }
        }
    }

    fn finalize(&mut self, handler: &mut dyn InboundMessageHandler) {
        for (entity, prop) in self.subscriptions.drain() {
            if let Err(e) = handler.unsubscribe(entity, &prop, self.self_key) {
                warn!(
                    "failed to unsubscribe from {:?}.{} during finalization of {:?}: {}",
                    entity, prop, self.self_key, e
                );
            }
        }
    }
}

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
            use slotmap::Key;
            let encoder = MockEncoder::new();
            let entities = mock_keys(1);
            let entity = entities[0];
            let obj_id = 1;
            let conn = ConnectionImpl {
                self_key: ConnectionKey::null(),
                encoder: Box::new(encoder.clone()),
                obj_map: Arc::new(MockObjectMap(entity, obj_id)),
                session: Mutex::new(Box::new(MockSession)),
                pending_get_requests: HashSet::new(),
                subscriptions: HashSet::new(),
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
            .property_value(test.entity, "foo", &Scalar(12.5), true)
            .expect("error updating property");
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Scalar(12.5))]
        );
    }
}
