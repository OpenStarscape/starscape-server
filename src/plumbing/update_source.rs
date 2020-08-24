use super::*;
use std::ops::Deref;

/// A value that produces updates whenever changed
/// Updates are not dispatched to connected properties immediatly, instead
///   property keys are stored until it is time to dispatch all updates
pub struct UpdateSource<T> {
    inner: T,
    subscribers: SubscriptionTracker,
}

impl<T> UpdateSource<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            subscribers: SubscriptionTracker::new(),
        }
    }

    /// Prefer set() where possible, because that can save work when value is unchanged
    pub fn get_mut(&mut self, pending: &PendingNotifications) -> &mut T {
        self.subscribers.queue_notifications(pending);
        &mut self.inner
    }

    /// This is useful, for example, when iterating over a slotmap and modifying elements,
    /// but not adding or removing them
    pub fn get_mut_without_notifying_of_change(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn subscribe(&self, subscriber: &Arc<dyn Subscriber>) -> Result<(), Box<dyn Error>> {
        match self.subscribers.subscribe(subscriber) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn unsubscribe(&self, subscriber: &Weak<dyn Subscriber>) -> Result<(), Box<dyn Error>> {
        match self.subscribers.unsubscribe(subscriber) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

impl<T: PartialEq> UpdateSource<T> {
    pub fn set(&mut self, pending: &PendingNotifications, value: T) {
        if self.inner != value {
            self.inner = value;
            self.subscribers.queue_notifications(pending);
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
    use crate::server::PropertyUpdateSink;
    use std::sync::RwLock;

    struct MockSubscriber;

    impl Subscriber for MockSubscriber {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn PropertyUpdateSink,
        ) -> Result<(), Box<dyn Error>> {
            panic!("MockSubscriber.notify() should not be called");
        }
    }

    fn setup() -> (UpdateSource<i64>, PendingNotifications, Arc<dyn Subscriber>) {
        let store = UpdateSource::new(7);
        let subscriber: Arc<dyn Subscriber> = Arc::new(MockSubscriber {});
        store.subscribe(&subscriber).expect("Failed to subscribe");
        (store, RwLock::new(Vec::new()), subscriber)
    }

    #[test]
    fn queues_notifications_when_set() {
        let (mut store, pending, _) = setup();
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 1);
    }

    #[test]
    fn queues_notifications_when_value_mut_accessed() {
        let (mut store, pending, _) = setup();
        store.get_mut(&pending);
        assert_eq!(pending.read().unwrap().len(), 1);
    }

    #[test]
    fn does_not_queue_notifications_on_get_mut_without_notifying_of_change() {
        let (mut store, pending, _) = setup();
        store.get_mut_without_notifying_of_change();
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn does_not_send_notification_when_set_to_same_value() {
        let (mut store, pending, _) = setup();
        store.set(&pending, 7);
        assert_eq!(pending.read().unwrap().len(), 0);
    }

    #[test]
    fn unsubscribing_stops_notifications() {
        let (mut store, pending, subscriber) = setup();
        store
            .unsubscribe(&Arc::downgrade(&subscriber))
            .expect("unsubbing failed");
        store.set(&pending, 5);
        assert_eq!(pending.read().unwrap().len(), 0);
    }
}
