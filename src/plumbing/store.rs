use std::error::Error;
use std::ops::Deref;
use std::sync::RwLock;

use crate::state::{PendingUpdates, PropertyKey};

/// A value that can be connected to 0, 1 or more properties
/// Updates are not dispatched to connected properties immediatly,
/// Property keys are stored until it is time to dispatch all updates
pub struct Store<T: PartialEq> {
    inner: T,
    // TODO: use an atomic bool to more quickly check if watchers is empty?
    /// The keys of watchers that want to be updated when value changes
    /// Is conceptually a set, but since length is almost always 0 or 1 use a low cost vec
    connections: RwLock<Vec<PropertyKey>>,
}

impl<T: PartialEq> Store<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            connections: RwLock::new(Vec::new()),
        }
    }

    /// Same as the Deref impl, but a named method can be easier to use sometimes
    pub fn value(&self) -> &T {
        &self.inner
    }

    pub fn set(&mut self, updates: &PendingUpdates, value: T) {
        if self.inner != value {
            self.inner = value;
            let conduits = self.connections.read().unwrap();
            if conduits.len() > 0 {
                let mut pending_updates =
                    updates.write().expect("Error writing to pending updates");
                pending_updates.extend(conduits.iter().cloned());
            }
        }
    }

    pub fn connect(&self, target: PropertyKey) -> Result<(), Box<Error>> {
        let mut connections = self.connections.write().unwrap();
        if connections.contains(&target) {
            Err(format!("already connected to {:?}", target).into())
        } else {
            connections.push(target);
            Ok(())
        }
    }

    pub fn disconnect(&self, target: PropertyKey) -> Result<(), Box<Error>> {
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

impl<T: PartialEq> Deref for Store<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mock_keys;
    use std::collections::HashSet;

    #[test]
    fn can_update_without_connected_properties() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        assert_eq!(*store, 7);
        store.set(&pending, 5);
        assert_eq!(*store, 5);
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn updates_connected_property_when_changed() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        let props = mock_keys(1);
        store.connect(props[0]).expect("connecting failed");
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 1);
        assert!(pending.read().unwrap().contains(&props[0]));
    }

    #[test]
    fn updates_multiple_connected_properties() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        let props = mock_keys(2);
        props
            .iter()
            .for_each(|p| store.connect(*p).expect("connecting failed"));
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 2);
        props
            .iter()
            .for_each(|p| assert!(pending.read().unwrap().contains(p)));
    }

    #[test]
    fn does_not_update_property_when_set_to_same_value() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        let props = mock_keys(1);
        store.connect(props[0]).expect("connecting failed");
        store.set(&pending, 7);
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn connecting_same_property_twice_errors() {
        let store = Store::new(7);
        let props = mock_keys(1);
        store.connect(props[0]).expect("connecting failed");
        assert!(store.connect(props[0]).is_err());
    }

    #[test]
    fn disconnecting_stops_updates() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        let props = mock_keys(2);
        props
            .iter()
            .for_each(|p| store.connect(*p).expect("connecting failed"));
        store.disconnect(props[1]).expect("disconnecting failed");
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 1);
        assert!(pending.read().unwrap().contains(&props[0]));
        assert!(!pending.read().unwrap().contains(&props[1]));
    }

    #[test]
    fn disconnecting_when_not_connected_errors() {
        let store = Store::new(7);
        let props = mock_keys(2);
        assert!(store.disconnect(props[0]).is_err());
        store.connect(props[0]).expect("connecting failed");
        assert!(store.disconnect(props[1]).is_err());
    }
}
