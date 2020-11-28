use super::*;
use crate::EntityKey;

pub trait PropertyUpdateSink {
    fn property_changed(
        &self,
        connection_key: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>>;
}

pub trait Server {
    fn process_requests(&mut self, handler: &mut dyn RequestHandler);
    fn number_of_connections(&self) -> usize;
    fn property_update_sink(&self) -> &dyn PropertyUpdateSink;
}
