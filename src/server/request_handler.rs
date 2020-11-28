use super::*;

/// A trait implemented by the state. The server uses this act on incoming client messages.
pub trait RequestHandler {
    fn set(&mut self, entity: EntityKey, property: &str, value: &Decodable) -> Result<(), String>;
    fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String>;
    fn subscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String>;
    fn unsubscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String>;
}
