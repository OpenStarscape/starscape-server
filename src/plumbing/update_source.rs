use std::error::Error;
use std::sync::RwLock;

use crate::state::{PendingUpdates, PropertyKey};

pub struct UpdateSource {
    // TODO: use an atomic bool to more quickly check if watchers is empty?
    /// The keys of watchers that want to be updated when value changes
    /// Is conceptually a set, but since length is almost always 0 or 1 use a low cost vec
    connections: RwLock<Vec<PropertyKey>>,
}

impl UpdateSource {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(Vec::new()),
        }
    }

    pub fn send_updates(&self, updates: &PendingUpdates) {
        let conduits = self.connections.read().unwrap();
        if conduits.len() > 0 {
            let mut pending_updates = updates.write().expect("Error writing to pending updates");
            pending_updates.extend(conduits.iter().cloned());
        }
    }

    pub fn connect(&self, target: PropertyKey) -> Result<(), Box<dyn Error>> {
        let mut connections = self.connections.write().unwrap();
        if connections.contains(&target) {
            Err(format!("already connected to {:?}", target).into())
        } else {
            connections.push(target);
            Ok(())
        }
    }

    pub fn disconnect(&self, target: PropertyKey) -> Result<(), Box<dyn Error>> {
        let mut connections = self.connections.write().unwrap();
        match connections.iter().position(|key| *key == target) {
            None => Err(format!("{:?} is not connected", target).into()),
            Some(i) => {
                connections.swap_remove(i);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mock_keys;
    use std::collections::HashSet;

    fn setup() -> (UpdateSource, PendingUpdates, Vec<PropertyKey>) {
        (
            UpdateSource::new(),
            RwLock::new(HashSet::new()),
            mock_keys(2),
        )
    }

    #[test]
    fn can_update_without_connected_properties() {
        let (source, pending, _) = setup();
        source.send_updates(&pending);
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn updates_multiple_connected_properties() {
        let (source, pending, props) = setup();
        props
            .iter()
            .for_each(|p| source.connect(*p).expect("connecting failed"));
        source.send_updates(&pending);
        assert_eq!(pending.read().unwrap().len(), 2);
        props
            .iter()
            .for_each(|p| assert!(pending.read().unwrap().contains(p)));
    }

    #[test]
    fn connecting_same_property_twice_errors() {
        let (source, _, props) = setup();
        source.connect(props[0]).expect("connecting failed");
        assert!(source.connect(props[0]).is_err());
    }

    #[test]
    fn disconnecting_stops_updates() {
        let (source, pending, props) = setup();
        props
            .iter()
            .for_each(|p| source.connect(*p).expect("connecting failed"));
        source.disconnect(props[1]).expect("disconnecting failed");
        source.send_updates(&pending);
        assert_eq!(pending.read().unwrap().len(), 1);
        assert!(pending.read().unwrap().contains(&props[0]));
        assert!(!pending.read().unwrap().contains(&props[1]));
    }

    #[test]
    fn disconnecting_when_not_connected_errors() {
        let (source, _, props) = setup();
        assert!(source.disconnect(props[0]).is_err());
        source.connect(props[0]).expect("connecting failed");
        assert!(source.disconnect(props[1]).is_err());
    }
}
