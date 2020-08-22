use std::{error::Error, sync::mpsc::Sender};

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

impl dyn Server {
    fn new_tcp_listener(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<Box<dyn Listener>, Box<dyn Error>> {
        let listener = TcpListener::new(new_session_tx, None, None)?;
        Ok(Box::new(listener))
    }

    pub fn new_impl(enable_tcp: bool) -> Box<dyn Server> {
        Box::new(ServerImpl::new(|new_session_tx| {
            let mut listeners: Vec<Box<dyn Listener>> = Vec::new();
            if enable_tcp {
                match Self::new_tcp_listener(new_session_tx) {
                    Ok(l) => listeners.push(l),
                    Err(e) => eprintln!("Failed to create TCP listener: {}", e),
                };
            }
            listeners
        }))
    }
}
