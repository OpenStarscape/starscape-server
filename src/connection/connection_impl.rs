use std::error::Error;
use std::io::Write;
use std::sync::Mutex;

use super::{Connection, ObjectId, ObjectMap, Protocol, Value};
use crate::state::{ConnectionKey, State};
use crate::EntityKey;

pub struct ConnectionImpl {
    self_key: ConnectionKey,
    protocol: Box<dyn Protocol>,
    objects: Mutex<ObjectMap>,
    writer: Mutex<Box<dyn Write>>,
}

impl ConnectionImpl {
    pub fn new(
        self_key: ConnectionKey,
        protocol: Box<dyn Protocol>,
        writer: Box<dyn Write>,
    ) -> Self {
        Self {
            self_key,
            protocol,
            objects: Mutex::new(ObjectMap::new()),
            writer: Mutex::new(writer),
        }
    }

    fn resolve_object(
        objects: &mut ObjectMap,
        entity: EntityKey,
        operation: &str,
    ) -> Result<ObjectId, Box<Error>> {
        match objects.get_object(entity) {
            Some(o_id) => Ok(o_id),
            None => Err(format!(
                "can not {}: {:?} does not have an object on this connection",
                operation, entity,
            )
            .into()),
        }
    }

    /// Returns a "resolved" value if needed, or None if the unresolved value is fine
    /// Entities and collections containing them need to be resolved to object IDs
    fn resolve_value<'a>(objects: &mut ObjectMap, unresolved_value: &'a Value) -> Option<Value> {
        match unresolved_value {
            // if the value is an entity, we need to look up the connection-specific object ID
            Value::Entity(entity) => Some(Value::Integer(match objects.get_object(*entity) {
                Some(o) => o,
                None => objects.register_entity(*entity),
            } as i64)),
            // if this value is a list, it could contain entities that should be resolved
            Value::List(list) => {
                // Search throug the list, looking for an element that need to be resolved
                match list.iter().enumerate().find_map(|(i, element)| {
                    Self::resolve_value(objects, element).map(|resolved| (i, resolved))
                }) {
                    // if we find one
                    Some((i, first_resolved)) => {
                        // clone the first part of the vec that didn't have any resolvable elements
                        let first_part = list.iter().take(i).cloned();
                        // insert the element we resolved
                        let with_first_resolved = first_part.chain(std::iter::once(first_resolved));
                        // and resolve or clone the rest of the vec as needed
                        let with_rest =
                            with_first_resolved.chain(list[i + 1..].iter().map(|element| {
                                Self::resolve_value(objects, element)
                                    .unwrap_or_else(|| element.clone())
                            }));
                        Some(Value::List(with_rest.collect()))
                    }
                    // otherwise, no elements need to be resolved
                    None => None,
                }
            }
            _ => None,
        }
    }

    fn write_buffer(&self, buffer: &[u8], operation: &str) -> Result<(), Box<Error>> {
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
        unresolved_value: &Value,
    ) -> Result<(), Box<Error>> {
        let operation = "update"; // used for error messages
        let resolved_value; // not used directly, only exists for lifetime reasons
        let (object, value) = {
            let mut objects = self.objects.lock().expect("Failed to read object map");
            let object = Self::resolve_object(&mut objects, entity, operation)?;
            resolved_value = Self::resolve_value(&mut objects, unresolved_value);
            let value = resolved_value.as_ref().unwrap_or(unresolved_value);
            (object, value)
        };
        let buffer = self
            .protocol
            .serialize_property_update(object, property, value)?;
        self.write_buffer(&buffer, operation)?;
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

    struct MockProtocol {
        log: Vec<(ObjectId, String, Value)>,
    }

    impl MockProtocol {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self { log: Vec::new() }))
        }
    }

    impl Protocol for Rc<RefCell<MockProtocol>> {
        fn serialize_property_update(
            &self,
            object: ObjectId,
            property: &str,
            value: &Value,
        ) -> Result<Vec<u8>, Box<Error>> {
            self.borrow_mut()
                .log
                .push((object, property.to_owned(), (*value).clone()));
            Ok(vec![])
        }
    }

    struct Test {
        proto: Rc<RefCell<MockProtocol>>,
        conn: ConnectionImpl,
        entity: EntityKey,
        obj_id: ObjectId,
        entities: Vec<EntityKey>,
    }

    impl Test {
        fn new() -> Self {
            let proto = MockProtocol::new();
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
            .property_changed(test.entity, "foo", &Value::Scaler(12.5))
            .expect("Error updating property");
        assert_eq!(
            test.proto.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Value::Scaler(12.5))]
        );
    }

    #[test]
    fn serializes_list_property_update() {
        let proto = MockProtocol::new();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(proto.clone()),
            Box::new(Vec::new()),
        );
        let e = mock_keys(1);
        let value = Value::List(vec![Value::Integer(7), Value::Integer(12)]);
        let o = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &value)
            .expect("Error updating property");
        assert_eq!(proto.borrow().log, vec![(o, "foo".to_owned(), value)],);
    }

    #[test]
    fn resolves_entity_value_to_object_id() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &Value::Entity(test.entities[0]))
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_ne!(test.obj_id, obj_0);
        assert_eq!(
            test.proto.borrow().log,
            vec![(test.obj_id, "foo".to_owned(), Value::Integer(obj_0 as i64))]
        );
    }

    #[test]
    fn resolves_the_same_entity_multiple_times() {
        let test = Test::new();
        test.conn
            .property_changed(test.entity, "foo", &Value::Entity(test.entities[0]))
            .expect("Error updating property");
        test.conn
            .property_changed(test.entity, "bar", &Value::Entity(test.entities[0]))
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_ne!(test.obj_id, obj_0);
        assert_eq!(
            test.proto.borrow().log,
            vec![
                (test.obj_id, "foo".to_owned(), Value::Integer(obj_0 as i64)),
                (test.obj_id, "bar".to_owned(), Value::Integer(obj_0 as i64))
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
                &Value::List(vec![
                    Value::Integer(42),
                    Value::Entity(test.entities[0]),
                    Value::Null,
                ]),
            )
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                Value::List(vec![
                    Value::Integer(42),
                    Value::Integer(obj_0 as i64),
                    Value::Null
                ])
            )],
        );
    }

    #[test]
    fn resolves_list_with_single_entity() {
        let test = Test::new();
        test.conn
            .property_changed(
                test.entity,
                "foo",
                &Value::List(vec![Value::Entity(test.entities[0])]),
            )
            .expect("Error updating property");
        let obj_0 = test.lookup_obj_0();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                Value::List(vec![Value::Integer(obj_0 as i64),])
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
                &Value::List(vec![
                    Value::List(vec![Value::List(vec![
                        Value::Entity(test.entities[1]),
                        Value::Entity(test.entities[2]),
                        Value::Null,
                    ])]),
                    Value::Integer(42),
                    Value::Entity(test.entities[0]),
                ]),
            )
            .expect("Error updating property");
        let obj_ids = test.lookup_obj_ids();
        assert_eq!(
            test.proto.borrow().log,
            vec![(
                test.obj_id,
                "foo".to_owned(),
                Value::List(vec![
                    Value::List(vec![Value::List(vec![
                        Value::Integer(obj_ids[1] as i64),
                        Value::Integer(obj_ids[2] as i64),
                        Value::Null,
                    ]),]),
                    Value::Integer(42),
                    Value::Integer(obj_ids[0] as i64),
                ]),
            )],
        );
    }
}
