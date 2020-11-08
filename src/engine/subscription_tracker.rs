use super::*;

/// Object that keeps track of and notifies a set of subscribers
pub struct SubscriptionTracker {
    subscribers: RwLock<Vec<(*const (), Weak<dyn Subscriber>)>>,
}

impl SubscriptionTracker {
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(Vec::new()),
        }
    }

    pub fn queue_notifications(&self, pending: &mut NotifQueue) {
        let subscribers = self.subscribers.read().expect("Failed to lock subscribers");
        if !subscribers.is_empty() {
            pending.extend(subscribers.iter().map(|(_ptr, sink)| sink.clone()));
        }
    }

    pub fn send_notifications(&self, state: &State, prop_update_sink: &dyn PropertyUpdateSink) {
        let subscribers = self.subscribers.read().expect("Failed to lock subscribers");
        for (_ptr, sink) in &*subscribers {
            if let Some(sink) = sink.upgrade() {
                if let Err(e) = sink.notify(state, prop_update_sink) {
                    eprintln!("Failed to process notification: {}", e);
                }
            } else {
                eprintln!(
                    "Failed to lock Weak; should have been unsubscribed before being dropped"
                );
            }
        }
    }

    // Returns true if there were no previous subscriptions
    pub fn subscribe(&self, subscriber: &Arc<dyn Subscriber>) -> Result<bool, String> {
        let mut subscribers = self
            .subscribers
            .write()
            .expect("Failed to lock subscribers");
        let subscriber = Arc::downgrade(subscriber);
        let subscriber_ptr = subscriber.thin_ptr();
        if subscribers
            .iter()
            .any(|(ptr, _sink)| *ptr == subscriber_ptr)
        {
            Err("Subscriber subscribed multiple times".into())
        } else {
            let was_empty = subscribers.is_empty();
            subscribers.push((subscriber_ptr, subscriber));
            Ok(was_empty)
        }
    }

    // Returns true if there are now no subscriptions
    pub fn unsubscribe(&self, subscriber: &Weak<dyn Subscriber>) -> Result<bool, String> {
        let subscriber_ptr = subscriber.thin_ptr();
        let mut subscribers = self
            .subscribers
            .write()
            .expect("Failed to lock subscribers");
        match subscribers
            .iter()
            .position(|(ptr, _sink)| *ptr == subscriber_ptr)
        {
            None => Err("Unsubscribed subscriber not already subscribed".into()),
            Some(i) => {
                subscribers.swap_remove(i);
                Ok(subscribers.is_empty())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use crate::server::ConnectionKey;
    use std::{cell::RefCell, error::Error};

    struct MockPropertyUpdateSink;

    impl PropertyUpdateSink for MockPropertyUpdateSink {
        fn property_changed(
            &self,
            _connection_key: ConnectionKey,
            _entity: EntityKey,
            _property: &str,
            _value: &Encodable,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    struct MockSubscriber(RefCell<u32>);

    impl Subscriber for MockSubscriber {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn PropertyUpdateSink,
        ) -> Result<(), Box<dyn Error>> {
            *self.0.borrow_mut() += 1;
            Ok(())
        }
    }

    fn setup() -> (
        SubscriptionTracker,
        NotifQueue,
        Vec<Arc<dyn Subscriber>>,
        Vec<Arc<MockSubscriber>>,
    ) {
        let mock_sinks: Vec<Arc<MockSubscriber>> = (0..3)
            .map(|_| Arc::new(MockSubscriber(RefCell::new(0))))
            .collect();
        (
            SubscriptionTracker::new(),
            Vec::new(),
            mock_sinks
                .iter()
                .map(|sink| sink.clone() as Arc<dyn Subscriber>)
                .collect(),
            mock_sinks,
        )
    }

    fn pending_contains(pending: &NotifQueue, sink: &Arc<dyn Subscriber>) -> bool {
        let sink = Arc::downgrade(&sink).thin_ptr();
        pending.iter().any(|i| i.thin_ptr() == sink)
    }

    #[test]
    fn can_queue_with_no_subscribers() {
        let (tracker, mut notifs, _, _) = setup();
        tracker.queue_notifications(&mut notifs);
        assert_eq!(notifs.len(), 0);
    }

    #[test]
    fn can_send_with_no_subscribers() {
        let (tracker, _, _, _) = setup();
        let state = State::new();
        let update_sink = MockPropertyUpdateSink {};
        tracker.send_notifications(&state, &update_sink);
    }

    #[test]
    fn queues_single_subscriber() {
        let (tracker, mut notifs, sinks, _) = setup();
        tracker
            .subscribe(&sinks[0].clone())
            .expect("subscribing failed");
        tracker.queue_notifications(&mut notifs);
        assert_eq!(notifs.len(), 1);
        assert!(pending_contains(&notifs, &sinks[0]));
    }

    #[test]
    fn sends_to_single_subscriber() {
        let (tracker, _, sinks, mock_sinks) = setup();
        tracker.subscribe(&sinks[0]).expect("subscribing failed");
        let state = State::new();
        let update_sink = MockPropertyUpdateSink {};
        tracker.send_notifications(&state, &update_sink);
        assert_eq!(*mock_sinks[0].0.borrow(), 1);
    }

    #[test]
    fn notifies_multiple_subscribers() {
        let (tracker, mut notifs, sinks, _) = setup();
        for sink in &sinks {
            tracker.subscribe(&sink).expect("subscribing failed");
        }
        tracker.queue_notifications(&mut notifs);
        assert_eq!(notifs.len(), 3);
        for sink in sinks {
            assert!(pending_contains(&notifs, &sink));
        }
    }

    #[test]
    fn subscribing_same_subscriber_twice_errors() {
        let (tracker, _, sinks, _) = setup();
        tracker.subscribe(&sinks[0]).expect("subscribing failed");
        assert!(tracker.subscribe(&sinks[0]).is_err());
    }

    #[test]
    fn unsubscribing_stops_notifications_queueing() {
        let (tracker, mut notifs, sinks, _) = setup();
        for sink in &sinks {
            tracker.subscribe(&sink).expect("subscribing failed");
        }
        tracker
            .unsubscribe(&Arc::downgrade(&sinks[1]))
            .expect("unsubscribing failed");
        tracker.queue_notifications(&mut notifs);
        assert_eq!(notifs.len(), 2);
        assert!(pending_contains(&notifs, &sinks[0]));
        assert!(!pending_contains(&notifs, &sinks[1]));
        assert!(pending_contains(&notifs, &sinks[2]));
    }

    #[test]
    fn unsubscribing_stops_notifications_sending() {
        let (tracker, _, sinks, mock_sinks) = setup();
        for sink in &sinks {
            tracker.subscribe(&sink).expect("subscribing failed");
        }
        tracker
            .unsubscribe(&Arc::downgrade(&sinks[1]))
            .expect("unsubscribing failed");
        let state = State::new();
        let update_sink = MockPropertyUpdateSink {};
        tracker.send_notifications(&state, &update_sink);
        assert_eq!(*mock_sinks[0].0.borrow(), 1);
        assert_eq!(*mock_sinks[1].0.borrow(), 0);
        assert_eq!(*mock_sinks[2].0.borrow(), 1);
    }

    #[test]
    fn unsubscribing_when_not_subscribed_errors() {
        let (tracker, _, sinks, _) = setup();
        assert!(tracker.unsubscribe(&Arc::downgrade(&sinks[0])).is_err());
        tracker.subscribe(&sinks[0]).expect("subscribing failed");
        assert!(tracker.unsubscribe(&Arc::downgrade(&sinks[1])).is_err());
    }
}
