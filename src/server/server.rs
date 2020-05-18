use super::*;
use crate::{EntityKey, State};

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
    fn apply_updates(&mut self, state: &mut State);
    fn property_update_sink(&self) -> &dyn PropertyUpdateSink;
}

impl dyn Server {
    pub fn new_impl() -> Box<dyn Server> {
        Box::new(ServerImpl::new())
    }
}
