use bimap::BiHashMap;
use serde::ser::{SerializeStruct, Serializer};
use std::io::Write;
use std::sync::Mutex;

//use crate::serialize::Serializable;
use crate::state::{ConnectionKey, EntityKey, State};

type ObjectId = u32;

pub struct Connection {
    writer: Mutex<Box<dyn Write>>,
    connection: ConnectionKey,
    objects: BiHashMap<EntityKey, ObjectId>,
    next_object_id: ObjectId,
}

impl Connection {
    pub fn new(connection: ConnectionKey, writer: Box<dyn Write>) -> Self {
        Self {
            writer: Mutex::new(writer),
            connection,
            objects: BiHashMap::new(),
            next_object_id: 1,
        }
    }

    fn serialize_property_update<T: serde::Serialize>(
        &self,
        serializer: &mut serde_json::Serializer<Vec<u8>>,
        object: &ObjectId,
        property: &str,
        value: &T,
    ) -> serde_json::error::Result<()> {
        let mut message = serializer.serialize_struct("Message", 4)?;
        message.serialize_field("mtype", "update")?;
        message.serialize_field("object", object)?;
        message.serialize_field("property", property)?;
        message.serialize_field("value", value)?;
        message.end()
    }

    pub fn send_property_update<T: serde::Serialize>(
        &self,
        entity: &EntityKey,
        property: &str,
        value: &T,
    ) -> Result<(), String> {
        let object = match self.objects.get_by_left(entity) {
            Some(o) => o,
            None => return Err("Object does not exist on this connection".to_owned()),
        };
        let buffer = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(buffer);
        match self.serialize_property_update(&mut serializer, object, property, value) {
            Ok(_) => {
                let mut writer = self.writer.lock().expect("Failed to lock writer");
                match writer.write(&serializer.into_inner()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Error writing to writer: {}", e)),
                }
            }
            Err(e) => Err(format!("{}", e)),
        }
    }

    pub fn register_object(&mut self, entity: EntityKey) -> u32 {
        let id = self.next_object_id;
        self.next_object_id += 1;
        self.objects.insert(entity, id);
        id
    }

    pub fn subscribe_to(&self, state: &State, object: u32, property: &str) {
        let entity = *self
            .objects
            .get_by_right(&object)
            .expect("Failed to look up object");
        let conduit = state.entities[entity]
            .conduit(property)
            .expect("Invalid property");
        if let Err(e) = state.conduits[conduit].subscribe(state, self.connection) {
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
