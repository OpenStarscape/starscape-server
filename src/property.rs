use std::ops::Deref;
use std::sync::RwLock;

use crate::state::{ConduitKey, PendingUpdates};

pub struct Property<T: PartialEq> {
    value: T,
    // TODO: use an atomic bool to more quickly check if watchers is empty?
    /// The keys of watchers that want to be updated when value changes
    /// Is conceptually a set, but since length is almost always 0 or 1 use a low cost vec
    conduits: RwLock<Vec<ConduitKey>>,
}

impl<T: PartialEq> Property<T> {
    pub fn new(value: T) -> Self {
        Property {
            value,
            conduits: RwLock::new(Vec::new()),
        }
    }

    pub fn set(&mut self, updates: &PendingUpdates, value: T) {
        if self.value != value {
            self.value = value;
            let conduits = self.conduits.read().unwrap();
            if conduits.len() > 0 {
                let mut pending_updates =
                    updates.write().expect("Error writing to pending updates");
                pending_updates.extend(conduits.iter().cloned());
            }
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn connect(&self, conduit: ConduitKey) {
        let mut conduits = self.conduits.write().unwrap();
        conduits.push(conduit);
        // TODO: error checking
    }

    pub fn disconnect(&self, conduit: ConduitKey) {
        let conduits = self.conduits.write().unwrap();
        // TODO
    }
}

impl<T: PartialEq> Deref for Property<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
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
