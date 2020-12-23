use super::*;

/// Returned by SubscriptionTracker::subscribe(), used instead of a raw bool for code readablity
pub struct SubscribeReport {
    pub was_empty: bool,
}

/// Returned by SubscriptionTracker::unsubscribe(), used instead of a raw bool for code readablity
pub struct UnsubscribeReport {
    pub is_now_empty: bool,
}

struct Inner {
    /// Store the thin pointers instead of getting them each time because getting required upgrading
    /// to an Arc
    subscribers: Vec<(*const (), Weak<dyn Subscriber>)>,
    notif_queue: Option<NotifQueue>,
}

impl Inner {
    pub fn subscribe(
        &mut self,
        subscriber: &Arc<dyn Subscriber>,
    ) -> Result<SubscribeReport, String> {
        let subscriber_ptr = subscriber.thin_ptr();
        let subscriber = Arc::downgrade(subscriber);
        if self
            .subscribers
            .iter()
            .any(|(ptr, _subscriber)| *ptr == subscriber_ptr)
        {
            Err("subscriber subscribed multiple times".into())
        } else {
            let was_empty = self.subscribers.is_empty();
            self.subscribers.push((subscriber_ptr, subscriber));
            Ok(SubscribeReport { was_empty })
        }
    }

    pub fn unsubscribe(
        &mut self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> Result<UnsubscribeReport, String> {
        let subscriber_ptr = subscriber.thin_ptr();
        match self
            .subscribers
            .iter()
            .position(|(ptr, _subscriber)| *ptr == subscriber_ptr)
        {
            None => Err("unsubscribed subscriber not already subscribed".into()),
            Some(i) => {
                self.subscribers.swap_remove(i);
                let is_now_empty = self.subscribers.is_empty();
                Ok(UnsubscribeReport { is_now_empty })
            }
        }
    }
}

/// Object that keeps track of a list of subscribers. Conceptually this is a set of Weaks, but you
/// can't hash or compare a weak so we use thin_ptr() for that. Since most lists will contain 0 or
/// 1 elements and iteration speed is most important, we use a Vec instead of a map.
pub struct SubscriptionTracker(RwLock<Inner>);

impl SubscriptionTracker {
    pub fn new() -> Self {
        Self(RwLock::new(Inner {
            subscribers: Vec::new(),
            notif_queue: None,
        }))
    }

    pub fn queue_notifications(&self) {
        let inner = self.0.read().expect("failed to lock subscribers");
        if !inner.subscribers.is_empty() {
            if let Some(notif_queue) = &inner.notif_queue {
                notif_queue.extend(
                    inner
                        .subscribers
                        .iter()
                        .map(|(_ptr, subscriber)| subscriber.clone()),
                );
            } else {
                error!("could not queue notifications because notification queue is unset");
            }
        }
    }

    pub fn send_notifications(
        &self,
        state: &State,
        prop_update_subscriber: &dyn OutboundMessageHandler,
    ) {
        let inner = self.0.read().expect("failed to lock subscribers");
        for (_ptr, subscriber) in &inner.subscribers {
            if let Some(subscriber) = subscriber.upgrade() {
                if let Err(e) = subscriber.notify(state, prop_update_subscriber) {
                    error!("failed to process notification: {}", e);
                }
            } else {
                error!("failed to lock Weak; should have been unsubscribed before being dropped");
            }
        }
    }

    pub fn subscribe_with_notif_queue(
        &self,
        subscriber: &Arc<dyn Subscriber>,
        notif_queue: &NotifQueue,
    ) -> Result<SubscribeReport, String> {
        let mut inner = self.0.write().expect("failed to lock subscribers");
        if let Some(prev) = &inner.notif_queue {
            if prev != notif_queue {
                let msg = "tried to subscribe with different notification queue";
                error!("{}", msg);
                return Err(msg.to_string());
            }
        } else {
            inner.notif_queue = Some(notif_queue.clone());
        }
        inner.subscribe(subscriber)
    }

    pub fn subscribe(&self, subscriber: &Arc<dyn Subscriber>) -> Result<SubscribeReport, String> {
        let mut inner = self.0.write().expect("failed to lock subscribers");
        inner.subscribe(subscriber)
    }

    pub fn unsubscribe(
        &self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> Result<UnsubscribeReport, String> {
        let mut inner = self.0.write().expect("failed to lock subscribers");
        inner.unsubscribe(subscriber)
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use std::{cell::RefCell, error::Error};

    struct MockSubscriber(RefCell<u32>);

    impl Subscriber for MockSubscriber {
        fn notify(&self, _: &State, _: &dyn OutboundMessageHandler) -> Result<(), Box<dyn Error>> {
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
        let mock_subscribers: Vec<Arc<MockSubscriber>> = (0..3)
            .map(|_| Arc::new(MockSubscriber(RefCell::new(0))))
            .collect();
        (
            SubscriptionTracker::new(),
            NotifQueue::new(),
            mock_subscribers
                .iter()
                .map(|subscriber| subscriber.clone() as Arc<dyn Subscriber>)
                .collect(),
            mock_subscribers,
        )
    }

    #[test]
    fn can_queue_with_no_subscribers() {
        let (tracker, notifs, _, _) = setup();
        tracker.queue_notifications();
        assert_eq!(notifs.len(), 0);
    }

    #[test]
    fn can_send_with_no_subscribers() {
        let (tracker, _, _, _) = setup();
        let state = State::new();
        let update_subscriber = MockOutboundMessageHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
    }

    #[test]
    fn queues_single_subscriber() {
        let (tracker, notifs, subscribers, _) = setup();
        tracker
            .subscribe_with_notif_queue(&subscribers[0].clone(), &notifs)
            .expect("subscribing failed");
        tracker.queue_notifications();
        assert_eq!(notifs.len(), 1);
        assert!(notifs.contains(&subscribers[0]));
    }

    #[test]
    fn sends_to_single_subscriber() {
        let (tracker, _, subscribers, mock_subscribers) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        let state = State::new();
        let update_subscriber = MockOutboundMessageHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
        assert_eq!(*mock_subscribers[0].0.borrow(), 1);
    }

    #[test]
    fn notifies_multiple_subscribers() {
        let (tracker, notifs, subscribers, _) = setup();
        for subscriber in &subscribers {
            tracker
                .subscribe_with_notif_queue(&subscriber, &notifs)
                .expect("subscribing failed");
        }
        tracker.queue_notifications();
        assert_eq!(notifs.len(), 3);
        for subscriber in subscribers {
            assert!(notifs.contains(&subscriber));
        }
    }

    #[test]
    fn subscribing_same_subscriber_twice_errors() {
        let (tracker, _, subscribers, _) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        assert!(tracker.subscribe(&subscribers[0]).is_err());
    }

    #[test]
    fn unsubscribing_stops_notifications_queueing() {
        let (tracker, notifs, subscribers, _) = setup();
        for subscriber in &subscribers {
            tracker
                .subscribe_with_notif_queue(&subscriber, &notifs)
                .expect("subscribing failed");
        }
        tracker
            .unsubscribe(&Arc::downgrade(&subscribers[1]))
            .expect("unsubscribing failed");
        tracker.queue_notifications();
        assert_eq!(notifs.len(), 2);
        assert!(notifs.contains(&subscribers[0]));
        assert!(!notifs.contains(&subscribers[1]));
        assert!(notifs.contains(&subscribers[2]));
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
        let update_subscriber = MockOutboundMessageHandler::new();
        tracker.send_notifications(&state, &update_subscriber);
        assert_eq!(*mock_subscribers[0].0.borrow(), 1);
        assert_eq!(*mock_subscribers[1].0.borrow(), 0);
        assert_eq!(*mock_subscribers[2].0.borrow(), 1);
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

    #[test]
    fn first_subscriber_reports_was_empty() {
        let (tracker, _, subscribers, _) = setup();
        let report = tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        assert_eq!(report.was_empty, true);
    }

    #[test]
    fn subsequent_subscribers_do_not_report_was_empty() {
        let (tracker, _, subscribers, _) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        let report = tracker
            .subscribe(&subscribers[1])
            .expect("subscribing failed");
        assert_eq!(report.was_empty, false);
        let report = tracker
            .subscribe(&subscribers[2])
            .expect("subscribing failed");
        assert_eq!(report.was_empty, false);
    }

    #[test]
    fn adding_removing_and_adding_new_subscriber_reports_was_empty() {
        let (tracker, _, subscribers, _) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        tracker
            .unsubscribe(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        let report = tracker
            .subscribe(&subscribers[1])
            .expect("subscribing failed");
        assert_eq!(report.was_empty, true);
    }

    #[test]
    fn removing_only_subscriber_reports_emtpy() {
        let (tracker, _, subscribers, _) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        let report = tracker
            .unsubscribe(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        assert_eq!(report.is_now_empty, true);
    }

    #[test]
    fn removing_one_of_two_subscribers_does_not_report_empty() {
        let (tracker, _, subscribers, _) = setup();
        tracker
            .subscribe(&subscribers[0])
            .expect("subscribing failed");
        tracker
            .subscribe(&subscribers[1])
            .expect("subscribing failed");
        let report = tracker
            .unsubscribe(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        assert_eq!(report.is_now_empty, false);
    }
}
