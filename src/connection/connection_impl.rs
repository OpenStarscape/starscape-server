use std::error::Error;
use std::io::Write;
use std::sync::Mutex;

use super::*;
use crate::state::{ConnectionKey, State};
use crate::EntityKey;

pub struct ConnectionImpl {
    self_key: ConnectionKey,
    encoder: Box<dyn Encoder>,
    objects: Mutex<ObjectMap>,
    writer: Mutex<Box<dyn Write>>,
}

impl ConnectionImpl {
    pub fn new(self_key: ConnectionKey, encoder: Box<dyn Encoder>, writer: Box<dyn Write>) -> Self {
        Self {
            self_key,
            encoder,
            objects: Mutex::new(ObjectMap::new()),
            writer: Mutex::new(writer),
        }
    }

    fn write_buffer(&self, buffer: &[u8], operation: &str) -> Result<(), Box<dyn Error>> {
        let mut writer = self.writer.lock().expect("Failed to lock writer");
        match writer.write(&buffer) {
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

    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str) {
        {
            let mut objects = self.objects.lock().expect("Failed to read object map");
            if objects.get_object(entity).is_none() {
                objects.register_entity(entity);
            }
        }
        let conduit = state.entities[entity]
            .property(property)
            .expect("Invalid property");
        if let Err(e) = state.properties[conduit].subscribe(state, self.self_key) {
            eprintln!("Error subscribing: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mock_keys;
    use slotmap::Key;
    use std::cell::RefCell;
    use std::rc::Rc;
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

    struct Test {
        proto: Rc<RefCell<MockEncoder>>,
        conn: ConnectionImpl,
        entity: EntityKey,
        obj_id: ObjectId,
        entities: Vec<EntityKey>,
    }

    impl Test {
        fn new() -> Self {
            let proto = MockEncoder::new();
            let conn = ConnectionImpl::new(
                ConnectionKey::null(),
                Box::new(proto.clone()),
                Box::new(Vec::new()),
            );
            let mut entities = mock_keys(4);
            let entity = entities.pop().unwrap();
            let obj_id = conn.objects.lock().unwrap().register_entity(entity);
            Self {
                proto,
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
            test.proto.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Scaler(12.5))]
        );
    }

    #[test]
    fn serializes_list_property_update() {
        let encoder = MockEncoder::new();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(encoder.clone()),
            Box::new(Vec::new()),
        );
        let e = mock_keys(1);
        let value = List(vec![Integer(7), Integer(12)]);
        let o = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &value)
            .expect("Error updating property");
        assert_eq!(encoder.borrow().log, vec![(o, "foo".to_owned(), value)],);
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
            test.proto.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Integer(obj_0 as i64))]
        );
    }

    #[test]
    fn resolves_the_same_entity_multiple_times() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &Entity(test.entities[0]))
            .expect("Error updating property");
        test.conn
            .property_changed(test.entity, "bar", &Entity(test.entities[0]))
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_ne!(test.obj_id, obj_0);
        assert_eq!(
            test.proto.borrow().log,
            vec![
                (test.obj_id, "foo".to_owned(), Integer(obj_0 as i64)),
                (test.obj_id, "bar".to_owned(), Integer(obj_0 as i64))
            ]
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
            test.proto.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), obj_ids.into())]
        );
    }

    #[test]
    fn resolves_list_with_entity_and_other_stuff() {
        let test = Test::new();
        test.conn
            .property_changed(
                test.entity,
                "foo",
                &List(vec![Integer(42), Entity(test.entities[0]), Null]),
            )
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                List(vec![Integer(42), Integer(obj_0 as i64), Null])
            )],
        );
    }

    #[test]
    fn resolves_list_with_single_entity() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &List(vec![Entity(test.entities[0])]))
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                List(vec![Integer(obj_0 as i64),])
            )],
        );
    }

    #[test]
    fn resolves_nested_lists_with_entities() {
        let test = Test::new();
        test.conn
            .property_changed(
                test.entity,
                "foo",
                &List(vec![
                    List(vec![List(vec![
                        Entity(test.entities[1]),
                        Entity(test.entities[2]),
                        Null,
                    ])]),
                    Integer(42),
                    Entity(test.entities[0]),
                ]),
            )
            .expect("Error updating property");
        let obj_ids = test.lookup_obj_ids();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                List(vec![
                    List(vec![List(vec![
                        Integer(obj_ids[1] as i64),
                        Integer(obj_ids[2] as i64),
                        Null,
                    ]),]),
                    Integer(42),
                    Integer(obj_ids[0] as i64),
                ]),
            )],
        );
    }
}
