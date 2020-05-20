use std::{
    error::Error,
    sync::{mpsc::Sender, Mutex},
};

use super::*;
use crate::{EntityKey, State};

struct IncomingDataHandler {
    connection_key: ConnectionKey,
    decoder: Box<dyn Decoder>,
    request_tx: Sender<Request>,
}

impl IncomingDataHandler {
    pub fn handle_incoming_data(&mut self, data: &[u8]) {
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
                eprintln!("Error decoding incoming data: {}", e);
                let _ = self
                    .request_tx
                    .send(Request::new(self.connection_key, ConnectionRequest::Close));
            }
        }
    }
}

pub struct ConnectionImpl {
    self_key: ConnectionKey,
    encoder: Box<dyn Encoder>,
    objects: Mutex<ObjectMap>,
    session: Mutex<Box<dyn Session>>,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        encoder: Box<dyn Encoder>,
        decoder: Box<dyn Decoder>,
        session_builder: Box<dyn SessionBuilder>,
        request_tx: Sender<Request>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut handler = IncomingDataHandler {
            connection_key: self_key,
            decoder,
            request_tx,
        };
        let session =
            session_builder.build(Box::new(move |data| handler.handle_incoming_data(data)))?;
        Ok(Self {
            self_key,
            encoder,
            objects: Mutex::new(ObjectMap::new()),
            session: Mutex::new(session),
        })
    }

    fn write_buffer(&self, buffer: &[u8], operation: &str) -> Result<(), Box<dyn Error>> {
        let mut session = self.session.lock().expect("Failed to lock writer");
        match session.send(&buffer) {
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
        unresolved_value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        let resolved_value; // not used directly, only exists for lifetime reasons
        let (object, value) = {
            let mut objects = self.objects.lock().expect("Failed to read object map");
            let object = objects.get_object(entity).ok_or_else(|| {
                format!(
                    "property_changed() with entity {:?} not in object map",
                    entity
                )
            })?;
            resolved_value = objects.resolve(unresolved_value);
            let value = resolved_value.as_ref().unwrap_or(unresolved_value);
            (object, value)
        };
        let buffer = self
            .encoder
            .encode_property_update(object, property, value)?;
        self.write_buffer(&buffer, "update")?;
        Ok(())
    }

    fn entity_destroyed(&self, _state: &State, entity: EntityKey) {
        self.objects
            .lock()
            .expect("Failed to write to object map")
            .remove_entity(entity);
        // TODO: tell client object was destroyed
    }

    fn object_to_entity(&self, object: ObjectId) -> Option<EntityKey> {
        self.objects
            .lock()
            .expect("Failed to write to object map")
            .get_entity(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mock_keys;
    use slotmap::Key;
    use std::{cell::RefCell, rc::Rc, sync::mpsc::channel};
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
            value: &Encodable,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            self.borrow_mut()
                .log
                .push((object, property.to_owned(), (*value).clone()));
            Ok(vec![])
        }
    }

    struct MockDecoder;

    impl Decoder for MockDecoder {
        fn decode(&mut self, _bytes: Vec<u8>) -> Result<Vec<ConnectionRequest>, Box<dyn Error>> {
            panic!("unexpected call");
        }
    }

    #[derive(Debug)]
    struct MockSession;

    impl Session for MockSession {
        fn send(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct MockSessionBuilder;

    impl SessionBuilder for MockSessionBuilder {
        fn build(
            self: Box<Self>,
            _handle_incoming_data: Box<dyn FnMut(&[u8]) -> () + Send>,
        ) -> Result<Box<dyn Session>, Box<dyn Error>> {
            Ok(Box::new(MockSession))
        }
    }

    struct Test {
        encoder: Rc<RefCell<MockEncoder>>,
        conn: ConnectionImpl,
        entity: EntityKey,
        obj_id: ObjectId,
        entities: Vec<EntityKey>,
    }

    impl Test {
        fn new() -> Self {
            let encoder = MockEncoder::new();
            let (tx, _) = channel();
            let conn = ConnectionImpl::new(
                ConnectionKey::null(),
                Box::new(encoder.clone()),
                Box::new(MockDecoder),
                Box::new(MockSessionBuilder),
                tx,
            )
            .expect("failed to construct connection");
            let mut entities = mock_keys(4);
            let entity = entities.pop().unwrap();
            let obj_id = conn.objects.lock().unwrap().register_entity(entity);
            Self {
                encoder,
                conn,
                entity,
                obj_id,
                entities,
            }
        }

        fn lookup_obj_0(&self) -> ObjectId {
            self.conn
                .objects
                .lock()
                .unwrap()
                .get_object(self.entities[0])
                .expect("failed to look up object")
        }

        fn lookup_obj_ids(&self) -> Vec<ObjectId> {
            self.entities
                .iter()
                .map(|e_key| {
                    self.conn
                        .objects
                        .lock()
                        .unwrap()
                        .get_object(*e_key)
                        .unwrap_or(0)
                })
                .collect()
        }
    }

    #[test]
    fn serializes_normal_property_update() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &Scaler(12.5))
            .expect("Error updating property");
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Scaler(12.5))]
        );
    }

    #[test]
    fn serializes_list_property_update() {
        let encoder = MockEncoder::new();
        let (tx, _) = channel();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(encoder.clone()),
            Box::new(MockDecoder),
            Box::new(MockSessionBuilder),
            tx,
        )
        .expect("failed to construct connection");
        let e = mock_keys(1);
        let value = List(vec![Integer(7), Integer(12)]);
        let o = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &value)
            .expect("Error updating property");
        assert_eq!(encoder.borrow().log, vec![(o, "foo".to_owned(), value)]);
    }

    #[test]
    fn resolves_entity_value_to_object_id() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &Entity(test.entities[0]))
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_ne!(test.obj_id, obj_0);
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Integer(obj_0 as i64))]
        );
    }

    #[test]
    fn resolves_list_of_entites_to_object_ids() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &test.entities.clone().into())
            .expect("Error updating property");
        let obj_ids = test.lookup_obj_ids();
        assert_eq!(
            test.encoder.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), obj_ids.into())]
        );
    }
}
