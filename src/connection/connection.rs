use super::*;

new_key_type! {
    /// A handle to a client connection
    pub struct ConnectionKey;
}

/// Manages a single client connection. Both the session type (TCP, WebRTC, etc) and the format
/// (JSON, etc) are abstracted.
pub trait Connection {
    fn property_changed(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
    fn entity_destroyed(&self, state: &State, entity: EntityKey);
    fn object_to_entity(&self, object: ObjectId) -> Option<EntityKey>;
}

/// Receives data from the session layer (on the session's thread), decodes it into requests and
/// sends those off to be processed by the session on the main thead.
struct InboundDataHandler {
    connection_key: ConnectionKey,
    decoder: Box<dyn Decoder>,
    request_tx: Sender<Request>,
}

impl InboundDataHandler {
    pub fn handle_data(&mut self, data: &[u8]) {
        match self.decoder.decode(data.to_owned()) {
            Ok(requests) => {
                requests.into_iter().for_each(|data| {
                    // dropping requests is fine if the channel is disconnected
                    let _ = self
                        .request_tx
                        .send(Request::new(self.connection_key, data));
                });
            }
            Err(e) => {
                warn!("can't decode incoming data: {}", e);
                let _ = self
                    .request_tx
                    .send(Request::new(self.connection_key, RequestType::Close));
            }
        }
    }
}

pub struct ConnectionImpl {
    encoder: Box<dyn Encoder>,
    obj_map: Box<dyn ObjectMap>,
    session: Mutex<Box<dyn Session>>,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        session_builder: Box<dyn SessionBuilder>,
        request_tx: Sender<Request>,
        encoder: Box<dyn Encoder>,
        decoder: Box<dyn Decoder>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut handler = InboundDataHandler {
            connection_key: self_key,
            decoder,
            request_tx,
        };
        let session = session_builder.build(Box::new(move |data| handler.handle_data(data)))?;
        Ok(Self {
            encoder,
            obj_map: Box::new(ObjectMapImpl::new()),
            session: Mutex::new(session),
        })
    }

    pub fn new_with_json(
        self_key: ConnectionKey,
        session_builder: Box<dyn SessionBuilder>,
        request_tx: Sender<Request>,
    ) -> Result<Self, Box<dyn Error>> {
        let (encoder, decoder) = json_protocol_impls();
        Self::new(self_key, session_builder, request_tx, encoder, decoder)
    }

    fn write_buffer(&self, buffer: &[u8], operation: &str) -> Result<(), Box<dyn Error>> {
        let mut session = self.session.lock().expect("failed to lock writer");
        match session.yeet_bundle(&buffer) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("can not {}; error writing to writer: {}", operation, e).into()),
        }
    }
}

impl Connection for ConnectionImpl {
    fn property_changed(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        let object = self.obj_map.get_object(entity).ok_or_else(|| {
            format!(
                "property_changed() with entity {:?} not in object map",
                entity
            )
        })?;
        let buffer = self.encoder.encode_property_update(
            object,
            property,
            self.obj_map.as_encode_ctx(),
            value,
        )?;
        self.write_buffer(&buffer, "update")?;
        Ok(())
    }

    fn entity_destroyed(&self, _state: &State, entity: EntityKey) {
        self.obj_map.remove_entity(entity);
        // TODO: tell client object was destroyed
    }

    fn object_to_entity(&self, object: ObjectId) -> Option<EntityKey> {
        self.obj_map.get_entity(object)
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
            let encoder = MockEncoder::new();
            let entities = mock_keys(1);
            let entity = entities[0];
            let obj_id = 1;
            let conn = ConnectionImpl {
                encoder: Box::new(encoder.clone()),
                obj_map: Box::new(MockObjectMap(entity, obj_id)),
                session: Mutex::new(Box::new(MockSession)),
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
            .property_changed(test.entity, "foo", &Scaler(12.5))
            .expect("error updating property");
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Scaler(12.5))]
        );
    }
}
