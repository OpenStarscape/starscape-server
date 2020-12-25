use super::*;

struct Events<T> {
    events: Vec<T>,
    notif_queue: Initializable<NotifQueue>,
}

struct Dispatcher<T> {
    events: Mutex<Events<T>>,
    /// Needs to be on a differnt lock than events so notify() doesn't deadlock
    subscribers: ConduitSubscriberList,
}

impl<T> Dispatcher<T> {
    fn new() -> Self {
        Self {
            events: Mutex::new(Events {
                events: Vec::new(),
                notif_queue: Initializable::new(),
            }),
            subscribers: ConduitSubscriberList::new(),
        }
    }
}

impl<T> Subscriber for Dispatcher<T> {
    fn notify(
        &self,
        state: &State,
        handler: &dyn OutboundMessageHandler,
    ) -> Result<(), Box<dyn Error>> {
        self.subscribers.send_notifications(state, handler);
        // now that the notifications have been processed we clear the pending events
        let mut events = self.events.lock().unwrap();
        events.events.clear();
        // Balence between not letting a very active tick consume a lot of memory indefinitely and
        // not requiring reallocation every time
        if events.events.capacity() > 10 {
            events.events.shrink_to_fit();
        }
        Ok(())
    }
}

pub enum EventsDontTakeInputSilly {}

impl<T: Clone> Conduit<Vec<T>, EventsDontTakeInputSilly> for Weak<Dispatcher<T>> {
    fn output(&self, _: &State) -> Result<Vec<T>, String> {
        let dispatcher = self.upgrade().ok_or("event no longer exists")?;
        let events = dispatcher.events.lock().unwrap();
        // We have to clone because there might be mutliple subscribers
        Ok(events.events.clone())
    }

    fn input(&self, _: &mut State, _: EventsDontTakeInputSilly) -> Result<(), String> {
        unreachable!();
    }

    fn subscribe(&self, _: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        let dispatcher = self.upgrade().ok_or("event no longer exists")?;
        dispatcher.subscribers.subscribe(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, _: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        let dispatcher = self.upgrade().ok_or("event no longer exists")?;
        dispatcher.subscribers.unsubscribe(subscriber)?;
        Ok(())
    }
}

/// Similar to `Element<T>`, except it allows the game to fire events.
pub struct EventElement<T> {
    dispatcher: Option<Arc<Dispatcher<T>>>,
}

impl<T: Clone + 'static> EventElement<T> {
    pub fn new() -> Self {
        Self { dispatcher: None }
    }

    /// Fire an event. Requires mut conceptually even though it could be implemented without it.
    pub fn fire(&mut self, data: T) {
        if let Some(dispatcher) = &self.dispatcher {
            let mut events = dispatcher.events.lock().unwrap();
            // Only add the dispatcher to the notification queue for the first event fired
            if events.events.is_empty() {
                match events.notif_queue.get() {
                    Ok(notif_queue) => notif_queue
                        .extend(std::iter::once(
                            Arc::downgrade(&dispatcher) as Weak<dyn Subscriber>
                        )),
                    Err(e) => error!("failed to fire event: {}", e),
                }
            }
            events.events.push(data);
        }
    }

    /// Send notifications to the given subscriber when the inner value changes
    pub fn conduit(
        &mut self,
        notif_queue: &NotifQueue,
    ) -> impl Conduit<Vec<T>, EventsDontTakeInputSilly> + Clone {
        if self.dispatcher.is_none() {
            self.dispatcher = Some(Arc::new(Dispatcher::new()));
        }
        let dispatcher = self.dispatcher.as_ref().unwrap();
        let mut events = dispatcher.events.lock().unwrap();
        if let Err(e) = events.notif_queue.try_init(notif_queue) {
            error!("problem creating event conduit: {}", e);
        }
        Arc::downgrade(&dispatcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_notifications(state: &State) {
        let mut buf = Vec::new();
        state.notif_queue.swap_buffer(&mut buf);
        let handler = MockOutboundMessageHandler::new();
        for notification in &buf {
            notification
                .upgrade()
                .expect("dead subscriber in notification queue")
                .notify(&state, &handler)
                .expect("notify failed");
        }
    }

    fn setup() -> (
        EventElement<i32>,
        State,
        impl Conduit<Vec<i32>, EventsDontTakeInputSilly> + Clone,
    ) {
        let mut event = EventElement::new();
        let state = State::new();
        let conduit = event.conduit(&state.notif_queue);
        (event, state, conduit)
    }

    fn checking_subscriber<C>(conduit: &C, expected: Vec<i32>) -> MockSubscriber
    where
        C: Conduit<Vec<i32>, EventsDontTakeInputSilly> + Clone + 'static,
    {
        let conduit = conduit.clone();
        MockSubscriber::new_with_fn(move |state| {
            assert_eq!(conduit.output(state), Ok(expected.clone()));
        })
    }

    #[test]
    fn can_fire_without_conduit() {
        let mut event = EventElement::new();
        event.fire(7);
    }

    #[test]
    fn can_fire_with_unsubscribed_conduit() {
        let (mut event, state, _) = setup();
        event.fire(7);
        send_notifications(&state);
    }

    #[test]
    fn subscribed_conduit_gets_notified() {
        let (mut event, state, conduit) = setup();
        let subscriber = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        event.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn sees_events_in_notification() {
        let (mut event, state, conduit) = setup();
        let subscriber = checking_subscriber(&conduit, vec![7, 3, 120]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        event.fire(7);
        event.fire(3);
        event.fire(120);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn can_have_multiple_subscribers() {
        let (mut event, state, conduit) = setup();
        let subscriber_a = MockSubscriber::new();
        let subscriber_b = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber_a.get()).unwrap();
        conduit.subscribe(&state, &subscriber_b.get()).unwrap();
        event.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber_a.notify_count(), 1);
        assert_eq!(subscriber_b.notify_count(), 1);
    }

    #[test]
    fn all_subscribers_see_event() {
        let (mut event, state, conduit) = setup();
        let subscriber_a = checking_subscriber(&conduit, vec![7]);
        let subscriber_b = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber_a.get()).unwrap();
        conduit.subscribe(&state, &subscriber_b.get()).unwrap();
        event.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber_a.notify_count(), 1);
        assert_eq!(subscriber_b.notify_count(), 1);
    }

    #[test]
    fn events_are_cleared_after_notifications_sent_without_subscriber() {
        let (mut event, state, conduit) = setup();
        event.fire(120);
        send_notifications(&state);
        event.fire(22);
        event.fire(9);
        send_notifications(&state);
        let subscriber = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        event.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn events_are_cleared_after_notifications_sent_with_subscriber() {
        let (mut event, state, conduit) = setup();
        let subscriber = checking_subscriber(&conduit, vec![7, 12]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        event.fire(7);
        event.fire(12);
        send_notifications(&state);
        event.fire(7);
        event.fire(12);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 2);
    }

    #[test]
    fn is_not_notified_on_no_events() {
        let (mut event, state, conduit) = setup();
        let subscriber = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        event.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
        event.fire(7);
        event.fire(12);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 2);
    }

    #[test]
    fn subscribing_late_still_sends_events() {
        let (mut event, state, conduit) = setup();
        event.fire(7);
        let subscriber = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }
}
