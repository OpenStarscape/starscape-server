use super::*;

/// An atomic unit of state. An element can be subscribed to, in which case it will notify the
/// subscriber when it is changed. These notifications are __not__ dispatched immediately. Instead,
/// they are queued and processed later in the main game loop.
pub struct Element<T> {
    inner: T,
    subscribers: SubscriptionTracker,
}

impl<T> Element<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            subscribers: SubscriptionTracker::new(),
        }
    }

    /// Prefer set() where possible. That can save work when value is unchanged.
    pub fn get_mut(&mut self) -> &mut T {
        self.subscribers.queue_notifications();
        &mut self.inner
    }

    /// This is useful, for example, when iterating over a slotmap and modifying elements,
    /// but not adding or removing them
    #[allow(dead_code)]
    pub fn get_mut_without_notifying_of_change(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Send notifications to the given subscriber when the inner value changes
    pub fn subscribe(
        &self,
        subscriber: &Arc<dyn Subscriber>,
        notif_queue: &NotifQueue,
    ) -> Result<(), Box<dyn Error>> {
        self.subscribers
            .subscribe_with_notif_queue(subscriber, notif_queue)?;
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber: &Weak<dyn Subscriber>) -> Result<(), Box<dyn Error>> {
        self.subscribers.unsubscribe(subscriber)?;
        Ok(())
    }
}

impl<T: PartialEq> Element<T> {
    pub fn set(&mut self, value: T) {
        if self.inner != value {
            self.inner = value;
            self.subscribers.queue_notifications();
        }
    }
}

impl<T> Deref for Element<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSubscriber;

    impl Subscriber for MockSubscriber {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn OutboundMessageHandler,
        ) -> Result<(), Box<dyn Error>> {
            panic!("MockSubscriber.notify() should not be called");
        }
    }

    fn setup() -> (Element<i64>, NotifQueue, Arc<dyn Subscriber>) {
        let store = Element::new(7);
        let subscriber: Arc<dyn Subscriber> = Arc::new(MockSubscriber {});
        let notif_queue = NotifQueue::new();
        store
            .subscribe(&subscriber, &notif_queue)
            .expect("failed to subscribe");
        (store, notif_queue, subscriber)
    }

    #[test]
    fn queues_notifications_when_set() {
        let (mut store, notifs, _) = setup();
        store.set(5);
        assert_eq!(notifs.len(), 1);
    }

    #[test]
    fn queues_notifications_when_value_mut_accessed() {
        let (mut store, notifs, _) = setup();
        store.get_mut();
        assert_eq!(notifs.len(), 1);
    }

    #[test]
    fn does_not_queue_notifications_on_get_mut_without_notifying_of_change() {
        let (mut store, notifs, _) = setup();
        store.get_mut_without_notifying_of_change();
        assert_eq!(notifs.len(), 0);
    }

    #[test]
    fn does_not_send_notification_when_set_to_same_value() {
        let (mut store, notifs, _) = setup();
        store.set(7);
        assert_eq!(notifs.len(), 0);
    }

    #[test]
    fn unsubscribing_stops_notifications() {
        let (mut store, notifs, subscriber) = setup();
        store
            .unsubscribe(&Arc::downgrade(&subscriber))
            .expect("unsubbing failed");
        store.set(5);
        assert_eq!(notifs.len(), 0);
    }
}
