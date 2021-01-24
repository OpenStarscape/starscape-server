use super::subscriber_list::{SubscribeReport, UnsubscribeReport};
use super::*;

/// A SubscriberList that is Sync (NOTE: is not currently sync because SubscriberList is not Send
/// because conduits are not Send because aaaahhhhh, but this will get fixed). Useful for sticking
/// in conduits that have to manage subscriptions in non-mut methods.
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

    /// Notify all subscribers
    pub fn send_notifications(&self, state: &State, handler: &dyn EventHandler) {
        if self.has_subscribers.load(SeqCst) {
            let lock = self.lock.lock().expect("failed to lock subscribers");
            for (_ptr, subscriber) in &lock.0 {
                if let Some(subscriber) = subscriber.upgrade() {
                    subscriber.notify(state, handler);
                } else {
                    error!(
                        "failed to lock Weak; should have been unsubscribed before being dropped"
                    );
                }
            }
        }
    }

    /// Add a subscriber
    pub fn subscribe(&self, subscriber: &Arc<dyn Subscriber>) -> RequestResult<SubscribeReport> {
        self.has_subscribers.store(true, SeqCst);
        let mut inner = self.lock.lock().expect("failed to lock subscribers");
        inner.subscribe(subscriber).map_err(|e| {
            if inner.0.is_empty() {
                self.has_subscribers.store(false, SeqCst);
            }
            e
        })
    }

    /// Remove a subscriber
    pub fn unsubscribe(
        &self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> RequestResult<UnsubscribeReport> {
        let mut inner = self.lock.lock().expect("failed to lock subscribers");
        let result = inner.unsubscribe(subscriber);
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

    #[test]
    fn can_send_with_no_subscribers() {
        let (tracker, _, _, _) = setup();
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
    }

    #[test]
    fn sends_to_single_subscriber() {
        let (tracker, _, subscribers, mock_subscribers) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
    }

    #[test]
    fn sends_to_multiple_subscribers() {
        let (tracker, _, subscribers, mock_subscribers) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        tracker
            .subscribe(&subscribers[1])
            .expect("subscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
        assert_eq!(mock_subscribers[1].notify_count(), 1);
    }

    #[test]
    fn unsubscribing_stops_notifications_sending() {
        let (tracker, _, subscribers, mock_subscribers) = setup();
        for subscriber in &subscribers {
            tracker.subscribe(&subscriber).expect("subscribing failed");
        }
        tracker
            .unsubscribe(&Arc::downgrade(&subscribers[1]))
            .expect("unsubscribing failed");
        let state = State::new();
        let update_subscriber = MockEventHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
        assert_eq!(mock_subscribers[0].notify_count(), 1);
        assert_eq!(mock_subscribers[1].notify_count(), 0);
        assert_eq!(mock_subscribers[2].notify_count(), 1);
    }

    #[test]
    fn unsubscribing_when_not_subscribed_errors() {
        let (tracker, _, subscribers, _) = setup();
        assert!(tracker
            .unsubscribe(&Arc::downgrade(&subscribers[0]))
            .is_err());
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        assert!(tracker
            .unsubscribe(&Arc::downgrade(&subscribers[1]))
            .is_err());
    }
}
