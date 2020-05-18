use slotmap::DenseSlotMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::*;
use crate::{EntityKey, State};

new_key_type! {
    pub struct ConnectionKey;
}

pub struct ServerImpl {
    connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
    _listeners: Vec<Box<dyn Listener>>,
    new_session_rx: Receiver<Box<dyn SessionBuilder>>,
}

impl ServerImpl {
    pub fn new<F>(build_listeners: F) -> Self
    where
        F: Fn(Sender<Box<dyn SessionBuilder>>) -> Vec<Box<dyn Listener>>,
    {
        let (new_session_tx, new_session_rx) = channel();
        ServerImpl {
            connections: DenseSlotMap::with_key(),
            _listeners: build_listeners(new_session_tx),
            new_session_rx,
        }
    }
}

impl ServerImpl {
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

    fn number_of_connections(&self) -> usize {
        self.connections.len()
    }

    fn property_update_sink(&self) -> &dyn PropertyUpdateSink {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, Weak};

    impl Session for () {
        fn send(&mut self, _data: &[u8]) -> Result<(), Box<dyn Error>> {
            panic!("Unecpected call");
        }
    }

    impl SessionBuilder for bool {
        fn build(
            self: Box<Self>,
            _handle_incoming_data: Box<dyn FnMut(&[u8]) -> () + Send>,
        ) -> Result<Box<dyn Session>, Box<dyn Error>> {
            if *self {
                Ok(Box::new(()))
            } else {
                Err("Session builder is supposed to error for test".into())
            }
        }
    }

    #[test]
    fn builds_and_holds_listeners() {
        impl Listener for Arc<()> {}
        let weak = {
            let (weak, _server) = {
                let arc = Arc::new(());
                let weak: Weak<()> = Arc::downgrade(&arc);
                let server = ServerImpl::new(|_| vec![Box::new(arc.clone())]);
                (weak, server)
            };
            assert_eq!(weak.strong_count(), 1);
            weak
        };
        assert_eq!(weak.strong_count(), 0);
    }

    #[test]
    fn has_no_connections_by_default() {
        let mut server = ServerImpl::new(|_| vec![]);
        assert_eq!(server.number_of_connections(), 0);
        let mut state = State::new();
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 0);
    }

    #[test]
    fn can_create_connection() {
        let new_session_tx = Mutex::new(None);
        let mut server = ServerImpl::new(|tx| {
            *new_session_tx.lock().unwrap() = Some(tx);
            vec![]
        });
        let builder: Box<dyn SessionBuilder> = Box::new(true);
        new_session_tx
            .lock()
            .unwrap()
            .as_ref()
            .expect("new_session_tx not set")
            .send(builder)
            .expect("failed to send connection builder");
        let mut state = State::new();
        assert_eq!(server.number_of_connections(), 0);
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 1);
    }

    #[test]
    fn does_not_create_connection_when_building_session_fails() {
        let new_session_tx = Mutex::new(None);
        let mut server = ServerImpl::new(|tx| {
            *new_session_tx.lock().unwrap() = Some(tx);
            vec![]
        });
        // Creating session will fails because false    vvvvv
        let builder: Box<dyn SessionBuilder> = Box::new(false);
        new_session_tx
            .lock()
            .unwrap()
            .as_ref()
            .expect("new_session_tx not set")
            .send(builder)
            .expect("failed to send connection builder");
        let mut state = State::new();
        server.apply_updates(&mut state);
        assert_eq!(server.number_of_connections(), 0);
    }
}
