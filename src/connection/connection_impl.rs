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
}

impl Connection for ConnectionImpl {
    fn property_changed(
        &self,
        entity: EntityKey,
        property: &str,
        unresolved_value: &Value,
    ) -> Result<(), Box<Error>> {
        let object: ObjectId;
        let resolved_value: Value;
        let value: &Value;
        {
            let mut objects = self.objects.lock().expect("Failed to read object map");
            object = match objects.get_object(entity) {
                Some(o) => o,
                None => {
                    return Err(format!(
                        "Updated entity {:?} does not have an object on this connection",
                        entity
                    )
                    .into())
                }
            };
            value = match unresolved_value {
                Value::Entity(entity) => {
                    resolved_value = Value::Integer(match objects.get_object(*entity) {
                        Some(o) => o,
                        None => objects.register_entity(*entity),
                    } as i64);
                    &resolved_value
                }
                value => value,
            };
        }
        match self
            .protocol
            .serialize_property_update(object, property, value)
        {
            Ok(buffer) => {
                let mut writer = self.writer.lock().expect("Failed to lock writer");
                match writer.write(&buffer) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error writing to writer: {}", e).into()),
                }
            }
            Err(e) => Err(format!("{}", e).into()),
        }
    }

    fn entity_destroyed(&self, state: &State, entity: EntityKey) {
        self.objects
            .lock()
            .expect("Failed to write to object map")
            .remove_entity(entity);
        // TODO: tell client object was destroyed
    }

    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str) {
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

    #[test]
    fn serializes_normal_property_update() {
        let proto = MockProtocol::new();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(proto.clone()),
            Box::new(Vec::new()),
        );
        let e = mock_keys(1);
        let o = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &Value::Scaler(12.5))
            .expect("Error updating property");
        assert_eq!(
            proto.borrow().log,
            vec![(o, "foo".to_owned(), Value::Scaler(12.5))]
        );
    }

    #[test]
    fn resolves_entity_value_to_object_id() {
        let proto = MockProtocol::new();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(proto.clone()),
            Box::new(Vec::new()),
        );
        let e = mock_keys(2);
        let o0 = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &Value::Entity(e[1]))
            .expect("Error updating property");
        let o1 = conn.objects.lock().unwrap().get_object(e[1]).unwrap();
        assert_ne!(o0, o1);
        assert_eq!(
            proto.borrow().log,
            vec![(o0, "foo".to_owned(), Value::Integer(o1 as i64))]
        );
    }

    #[test]
    fn resolves_the_same_entity_multiple_times() {
        let proto = MockProtocol::new();
        let conn = ConnectionImpl::new(
            ConnectionKey::null(),
            Box::new(proto.clone()),
            Box::new(Vec::new()),
        );
        let e = mock_keys(2);
        let o0 = conn.objects.lock().unwrap().register_entity(e[0]);
        conn.property_changed(e[0], "foo", &Value::Entity(e[1]))
            .expect("Error updating property");
        conn.property_changed(e[0], "bar", &Value::Entity(e[1]))
            .expect("Error updating property");
        let o1 = conn.objects.lock().unwrap().get_object(e[1]).unwrap();
        assert_ne!(o0, o1);
        assert_eq!(
            proto.borrow().log,
            vec![
                (o0, "foo".to_owned(), Value::Integer(o1 as i64)),
                (o0, "bar".to_owned(), Value::Integer(o1 as i64))
            ]
        );
    }
}
