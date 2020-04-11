use bimap::BiHashMap;
use serde::ser::{Serialize, Serializer};
use std::error::Error;
use std::io::Write;
use std::sync::{Mutex, RwLock};

use crate::state::{ConnectionKey, EntityKey, State};
use crate::value::Value;

type ObjectId = u64;

pub trait Connection {
    fn send_property_update(
        &self,
        entity: EntityKey,
        property: &str,
        value: &Value,
    ) -> Result<(), Box<Error>>;
    fn register_object(&self, entity: EntityKey);
    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str);
}

struct ObjectMap {
    map: BiHashMap<EntityKey, ObjectId>,
    next_id: ObjectId,
}

pub struct JsonConnection {
    writer: Mutex<Box<dyn Write>>,
    connection: ConnectionKey,
    objects: RwLock<ObjectMap>,
}

impl JsonConnection {
    pub fn new(connection: ConnectionKey, writer: Box<dyn Write>) -> Self {
        JsonConnection {
            writer: Mutex::new(writer),
            connection,
            objects: RwLock::new(ObjectMap {
                map: BiHashMap::new(),
                next_id: 1,
            }),
        }
    }

    fn serialize_property_update<T: serde::Serialize>(
        &self,
        serializer: &mut serde_json::Serializer<Vec<u8>>,
        object: &ObjectId,
        property: &str,
        value: &T,
    ) -> serde_json::error::Result<()> {
        use serde::ser::SerializeStruct;
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "update")?;
        message.serialize_field("object", object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", value)?;
        message.end()
    }
}

impl Connection for JsonConnection {
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
            object = match objects.map.get_by_left(&entity) {
                Some(o) => *o,
                None => {
                    return Err(format!(
                        "Updated entity {:?} does not have an object on this connection",
                        entity
                    )
                    .into())
                }
            };
            value = match unresolved_value {
                Value::Entity(entity) => match objects.map.get_by_left(entity) {
                    Some(o) => {
                        resolved_value = Value::Integer(*o as i64);
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
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        match self.serialize_property_update(&mut serializer, &object, property, value) {
            Ok(_) => {
                let mut writer = self.writer.lock().expect("Failed to lock writer");
                match writer.write(&serializer.into_inner()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error writing to writer: {}", e).into()),
                }
            }
            Err(e) => Err(format!("{}", e).into()),
        }
    }

    fn register_object(&self, entity: EntityKey) {
        let mut objects = self.objects.write().expect("Failed to write to object map");
        let id = objects.next_id;
        objects.next_id += 1;
        objects.map.insert(entity, id);
    }

    fn subscribe_to(&self, state: &State, entity: EntityKey, property: &str) {
        let conduit = state.entities[entity]
            .conduit(property)
            .expect("Invalid property");
        if let Err(e) = state.conduits[conduit].subscribe(state, self.connection) {
            eprintln!("Error subscribing: {}", e);
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Vector(vector) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(3)?;
                tuple.serialize_element(&vector.x)?;
                tuple.serialize_element(&vector.y)?;
                tuple.serialize_element(&vector.z)?;
                tuple.end()
            }
            Value::Scaler(value) => serializer.serialize_f64(*value),
            Value::Integer(value) => serializer.serialize_i64(*value),
            Value::Entity(entity) => {
                panic!(
                    "Can not serialize {:?}; entity should have been replaced by object ID",
                    entity
                );
            }
            Value::Null => serializer.serialize_none(),
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
