use std::error::Error;
use std::ops::Deref;

use super::UpdateSource;
use crate::state::{PendingUpdates, PropertyKey};

/// A value that can be connected to 0, 1 or more properties
/// Updates are not dispatched to connected properties immediatly,
/// Property keys are stored until it is time to dispatch all updates
pub struct Store<T: PartialEq> {
    inner: T,
    update_source: UpdateSource,
}

impl<T: PartialEq> Store<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            update_source: UpdateSource::new(),
        }
    }

    /// Same as the Deref impl, but a named method can be easier to use sometimes
    pub fn value(&self) -> &T {
        &self.inner
    }

    pub fn set(&mut self, updates: &PendingUpdates, value: T) {
        if self.inner != value {
            self.inner = value;
            self.update_source.send_updates(updates);
        }
    }

    pub fn connect(&self, target: PropertyKey) -> Result<(), Box<Error>> {
        self.update_source.connect(target)
    }

    pub fn disconnect(&self, target: PropertyKey) -> Result<(), Box<Error>> {
        self.update_source.disconnect(target)
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
    use std::sync::RwLock;

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
    fn does_not_update_property_when_set_to_same_value() {
        let mut store = Store::new(7);
        let pending = RwLock::new(HashSet::new());
        let props = mock_keys(1);
        store.connect(props[0]).expect("connecting failed");
        store.set(&pending, 7);
        assert_eq!(pending.read().unwrap().len(), 0);
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

}
