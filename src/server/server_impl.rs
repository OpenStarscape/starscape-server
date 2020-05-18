use slotmap::DenseSlotMap;
use std::sync::mpsc::{channel, Receiver};

use super::*;
use crate::{EntityKey, State};

new_key_type! {
    pub struct ConnectionKey;
}

pub struct ServerImpl {
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    listeners: Vec<Box<dyn Listener>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
}

impl ServerImpl {
    pub fn new() -> Self {
        let (new_session_tx, new_session_rx) = channel();
        let mut listeners: Vec<Box<dyn Listener>> = Vec::new();
        match TcpListener::new(new_session_tx, None, None) {
            Ok(l) => listeners.push(Box::new(l)),
            Err(e) => eprintln!("Failed to create TCP server: {}", e),
        };
        ServerImpl {
            connections: DenseSlotMap::with_key(),
            listeners,
            new_session_rx,
        }
    }
}

impl ServerImpl {
    // TODO: test this
    fn try_build_connection(&mut self, builder: Box<dyn SessionBuilder>) {
        eprintln!("New session: {:?}", builder);
        // hack to get around slotmap only giving us a key after creation
        let key = self.connections.insert(Box::new(()));
        let (encoder, decoder) = json_protocol_impls();
        match ConnectionImpl::new(key, encoder, decoder, builder) {
            Ok(c) => {
                self.connections[key] = Box::new(c);
            }
            Err(e) => {
                self.connections.remove(key);
                eprintln!("Error building connection: {}", e);
            }
        }
    }
}

impl PropertyUpdateSink for ServerImpl {
    fn property_changed(
        &self,
        connection_key: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(connection) = self.connections.get(connection_key) {
            connection.property_changed(entity, property, &value)?;
            Ok(())
        } else {
            Err(format!("connection {:?} has died", connection_key).into())
        }
    }
}

impl Server for ServerImpl {
    fn apply_updates(&mut self, _state: &mut State) {
        while let Ok(session_builder) = self.new_session_rx.try_recv() {
            self.try_build_connection(session_builder);
        }
    }

    fn property_update_sink(&self) -> &dyn PropertyUpdateSink {
        self
    }
}
