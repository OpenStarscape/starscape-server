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

    pub fn connect(&self, target: PropertyKey) {
        let mut connections = self.connections.write().unwrap();
        connections.push(target);
        // TODO: error checking
    }

    pub fn disconnect(&self, target: PropertyKey) {
        let connections = self.connections.write().unwrap();
        // TODO
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

    #[test]
    fn does_not_update_subscribers_when_set_to_same_value() {
        panic!("Test not implemented");
    }

    #[test]
    fn set_does_not_serialize_when_there_are_no_subscribers() {
        panic!("Test not implemented");
    }

    #[test]
    fn set_does_not_serialize_when_all_subscribers_removed() {
        panic!("Test not implemented");
    }

    #[test]
    fn first_subscriber_creates_coupling() {
        panic!("Test not implemented");
    }

    #[test]
    fn multiple_subscribers_add_to_coupling() {
        panic!("Test not implemented");
    }

    #[test]
    fn removing_only_subscriber_removes_coupling() {
        panic!("Test not implemented");
    }

    #[test]
    fn removing_all_subscribers_removes_coupling() {
        panic!("Test not implemented");
    }

    #[test]
    fn subscribing_twice_errors_or_something() {
        panic!("Test not implemented");
    }

    #[test]
    fn unsubscribing_when_not_subscribed_errors_or_something() {
        panic!("Test not implemented");
    }
}
