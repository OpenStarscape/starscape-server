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
    fn number_of_connections(&self) -> usize;
    fn property_update_sink(&self) -> &dyn PropertyUpdateSink;
}

impl dyn Server {
    pub fn new_impl(enable_tcp: bool) -> Box<dyn Server> {
        Box::new(ServerImpl::new(|new_session_tx| {
            let mut listeners: Vec<Box<dyn Listener>> = Vec::new();
            if enable_tcp {
                match TcpListener::new(new_session_tx, None, None) {
                    Ok(l) => listeners.push(Box::new(l)),
                    Err(e) => eprintln!("Failed to create TCP server: {}", e),
                };
            }
            listeners
        }))
    }
}
