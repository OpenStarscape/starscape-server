use super::subscriber_list::{SubscribeReport, UnsubscribeReport};
use super::*;

/// A SubscriberList that is Sync. Useful for sticking in conduits that have to manage subscriptions in non-mut methods.
pub struct SyncSubscriberList {
    lock: Mutex<SubscriberList>,
    /// Always true when the subscriber list has subscribers. Can be briefly true when the inner
    /// subscriber does not have subscribers. Slightly speeds up the case of no subscribers.
    /// Absolutely a premature optimization.
    has_subscribers: AtomicBool,
}

impl AssertIsSync for SyncSubscriberList {}

impl SyncSubscriberList {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(SubscriberList::new()),
            has_subscribers: AtomicBool::new(false),
        }
    }

    /// Call the given function for each added subscriber weak (they should all be alive but logic errors could cause
    /// them not to be)
    pub fn for_each_weak_subscriber<F: FnMut(&Weak<dyn Subscriber>)>(&self, mut f: F) {
        if self.has_subscribers.load(SeqCst) {
            let lock = self.lock.lock().expect("failed to lock subscribers");
            for (_ptr, subscriber) in &lock.0 {
                f(subscriber);
            }
        }
    }

    /// Call the given function for each added subscriber
    pub fn for_each_subscriber<F: FnMut(Arc<dyn Subscriber>)>(&self, mut f: F) {
        self.for_each_weak_subscriber(|w| {
            if let Some(s) = w.upgrade() {
                f(s);
            } else {
                error!("failed to lock Weak; should have been unsubscribed before being dropped");
            }
        });
    }

    /// Notify all subscribers
    pub fn send_notifications(&self, state: &State, handler: &dyn EventHandler) {
        self.for_each_subscriber(|s| {
            s.notify(state, handler);
        });
    }

    /// Subscribe all added subscribers to the target subscribable
    pub fn subscribe_all(&self, state: &State, target: &dyn Subscribable) {
        self.for_each_subscriber(|s| {
            target
                .subscribe(state, &s)
                .or_log_error("subscribing subscriber list");
        });
    }

    /// Unsubscribe all added subscribers from the target subscribable
    pub fn unsubscribe_all(&self, state: &State, target: &dyn Subscribable) {
        self.for_each_weak_subscriber(|w| {
            target
                .unsubscribe(state, w)
                .or_log_error("unsubscribing subscriber list");
        });
    }

    /// Add a subscriber
    pub fn add(&self, subscriber: &Arc<dyn Subscriber>) -> RequestResult<SubscribeReport> {
        self.has_subscribers.store(true, SeqCst);
        let mut inner = self.lock.lock().expect("failed to lock subscribers");
        inner.add(subscriber).map_err(|e| {
            if inner.0.is_empty() {
                self.has_subscribers.store(false, SeqCst);
            }
            e
        })
    }

    /// Remove a subscriber
    pub fn remove(&self, subscriber: &Weak<dyn Subscriber>) -> RequestResult<UnsubscribeReport> {
        let mut inner = self.lock.lock().expect("failed to lock subscribers");
        let result = inner.remove(subscriber);
        if let Ok(report) = &result {
            if report.is_now_empty {
                self.has_subscribers.store(false, SeqCst);
            }
        };
        result
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;

    fn setup() -> (
        SyncSubscriberList,
        NotifQueue,
        Vec<Arc<dyn Subscriber>>,
        Vec<MockSubscriber>,
    ) {
        let mock_subscribers: Vec<MockSubscriber> = (0..3).map(|_| MockSubscriber::new()).collect();
        (
            SyncSubscriberList::new(),
            NotifQueue::new(),
            mock_subscribers.iter().map(|s| s.get()).collect(),
            mock_subscribers,
        )
    }

    // TODO: test subscribe_all/unsubscribe_all

    #[test]
    fn loops_through_all_weak_subscribers() {
        let (list, _, subscribers, _) = setup();
        list.add(&subscribers[0]).unwrap();
        list.add(&subscribers[1]).unwrap();
        let mut count = 0;
        list.for_each_weak_subscriber(|w| {
            assert_eq!(
                w.upgrade().unwrap().thin_ptr(),
                subscribers[count].thin_ptr()
            );
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn loops_through_all_subscribers() {
        let (list, _, subscribers, _) = setup();
        list.add(&subscribers[0]).unwrap();
        list.add(&subscribers[1]).unwrap();
        let mut count = 0;
        list.for_each_subscriber(|s| {
            assert_eq!(s.thin_ptr(), subscribers[count].thin_ptr());
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn ignores_dead_subscribers_when_looping() {
        let (list, _, subscribers, _) = setup();
        list.add(&subscribers[0]).unwrap();
        list.add(&Arc::new(MockSubscriber::new().get())).unwrap();
        list.add(&subscribers[1]).unwrap();
        let mut count = 0;
        list.for_each_subscriber(|s| {
            assert_eq!(s.thin_ptr(), subscribers[count].thin_ptr());
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn can_send_with_no_subscribers() {
        let (list, _, _, _) = setup();
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        list.send_notifications(&state, &update_subscriber);
    }

    #[test]
    fn sends_to_single_subscriber() {
        let (list, _, subscribers, mock_subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        list.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
    }

    #[test]
    fn sends_to_multiple_subscribers() {
        let (list, _, subscribers, mock_subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        list.add(&subscribers[1]).expect("subscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        list.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
        assert_eq!(mock_subscribers[1].notify_count(), 1);
    }

    #[test]
    fn unsubscribing_stops_notifications_sending() {
        let (list, _, subscribers, mock_subscribers) = setup();
        for subscriber in &subscribers {
            list.add(&subscriber).expect("subscribing failed");
        }
        list.remove(&Arc::downgrade(&subscribers[1]))
            .expect("unsubscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        list.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
        assert_eq!(mock_subscribers[1].notify_count(), 0);
        assert_eq!(mock_subscribers[2].notify_count(), 1);
    }

    #[test]
    fn unsubscribing_when_not_subscribed_errors() {
        let (list, _, subscribers, _) = setup();
        assert!(list.remove(&Arc::downgrade(&subscribers[0])).is_err());
        list.add(&subscribers[0]).expect("subscribing failed");
        assert!(list.remove(&Arc::downgrade(&subscribers[1])).is_err());
    }
}
