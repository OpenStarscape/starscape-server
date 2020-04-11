use std::error::Error;
use std::io::Write;
use std::sync::{Mutex, RwLock};

use super::{Connection, ObjectId, ObjectMap, Protocol, Value};
use crate::state::{ConnectionKey, State};
use crate::EntityKey;

pub struct ConnectionImpl {
    self_key: ConnectionKey,
    protocol: Box<dyn Protocol>,
    objects: RwLock<ObjectMap>,
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
            objects: RwLock::new(ObjectMap::new()),
            writer: Mutex::new(writer),
        }
    }
}

impl Connection for ConnectionImpl {
    fn send_property_update(
        &self,
        entity: EntityKey,
        property: &str,
        unresolved_value: &Value,
    ) -> Result<(), Box<Error>> {
        let object: ObjectId;
        let resolved_value: Value;
        let value: &Value;
        {
            let objects = self.objects.read().expect("Failed to read object map");
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
                Value::Entity(entity) => match objects.get_object(*entity) {
                    Some(o) => {
                        resolved_value = Value::Integer(o as i64);
                        &resolved_value
                    }
                    None => {
                        return Err(format!(
                            "Referenced entity {:?} does not have an object on this connection",
                            entity
                        )
                        .into())
                    }
                },
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

    fn register_object(&self, entity: EntityKey) {
        self.objects
            .write()
            .expect("Failed to write to object map")
            .register_entity(entity);
    }

    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str) {
        let conduit = state.entities[entity]
            .conduit(property)
            .expect("Invalid property");
        if let Err(e) = state.conduits[conduit].subscribe(state, self.self_key) {
            eprintln!("Error subscribing: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_ids_count_up_from_1() {
        panic!("Test not implemented");
    }
}
