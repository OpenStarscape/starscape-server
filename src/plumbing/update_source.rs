use std::error::Error;
use std::ops::Deref;

use super::NotificationSource;
use crate::state::{PendingUpdates, PropertyKey};

/// A value that produces updates whenever changed
/// Updates are not dispatched to connected properties immediatly, instead
///   property keys are stored until it is time to dispatch all updates
pub struct UpdateSource<T> {
    inner: T,
    notification_source: NotificationSource,
}

impl<T> UpdateSource<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            notification_source: NotificationSource::new(),
        }
    }

    /// Prefer set() where possible, because that can save work when value is unchanged
    pub fn get_mut(&mut self, updates: &PendingUpdates) -> &mut T {
        self.notification_source.send_updates(updates);
        &mut self.inner
    }

    /// This is useful, for example, when iterating over a slotmap and modifying elements,
    /// but not adding or removing them
    pub fn get_mut_without_sending_updates(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn connect(&self, target: PropertyKey) -> Result<(), Box<dyn Error>> {
        self.notification_source.connect(target)
    }

    pub fn disconnect(&self, target: PropertyKey) -> Result<(), Box<dyn Error>> {
        self.notification_source.disconnect(target)
    }
}

impl<T: PartialEq> UpdateSource<T> {
    pub fn set(&mut self, updates: &PendingUpdates, value: T) {
        if self.inner != value {
            self.inner = value;
            self.notification_source.send_updates(updates);
        }
    }
}

impl<T> Deref for UpdateSource<T> {
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

    fn setup() -> (UpdateSource<i64>, PendingUpdates, Vec<PropertyKey>) {
        (
            UpdateSource::new(7),
            RwLock::new(HashSet::new()),
            mock_keys(2),
        )
    }

    #[test]
    fn updates_connected_property_when_changed() {
        let (mut store, pending, props) = setup();
        store.connect(props[0]).expect("connecting failed");
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 1);
        assert!(pending.read().unwrap().contains(&props[0]));
    }

    #[test]
    fn always_updates_connected_property_when_value_mut_accessed() {
        let (mut store, pending, props) = setup();
        store.connect(props[0]).expect("connecting failed");
        store.get_mut(&pending);
        assert_eq!(pending.read().unwrap().len(), 1);
        assert!(pending.read().unwrap().contains(&props[0]));
    }

    #[test]
    fn does_not_send_updates_on_get_mut_without_sending_updates() {
        let (mut store, pending, props) = setup();
        store.connect(props[0]).expect("connecting failed");
        store.get_mut_without_sending_updates();
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn does_not_update_property_when_set_to_same_value() {
        let (mut store, pending, props) = setup();
        store.connect(props[0]).expect("connecting failed");
        store.set(&pending, 7);
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn disconnecting_stops_updates() {
        let (mut store, pending, props) = setup();
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
