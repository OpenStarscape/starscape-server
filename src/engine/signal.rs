use super::*;

struct Signals<T> {
    signals: Vec<T>,
    notif_queue: Initializable<NotifQueue>,
}

struct Dispatcher<T> {
    signals: Mutex<Signals<T>>,
    /// Needs to be on a differnt lock than signals so notify() doesn't deadlock
    subscribers: ConduitSubscriberList,
}

impl<T> Dispatcher<T> {
    fn new() -> Self {
        Self {
            signals: Mutex::new(Signals {
                signals: Vec::new(),
                notif_queue: Initializable::new(),
            }),
            subscribers: ConduitSubscriberList::new(),
        }
    }
}

impl<T> Subscriber for Dispatcher<T> {
    fn notify(&self, state: &State, handler: &dyn EventHandler) -> Result<(), Box<dyn Error>> {
        self.subscribers.send_notifications(state, handler);
        // now that the notifications have been processed we clear the pending signals
        let mut signals = self.signals.lock().unwrap();
        signals.signals.clear();
        // Balence between not letting a very active tick consume a lot of memory indefinitely and
        // not requiring reallocation every time
        if signals.signals.capacity() > 10 {
            signals.signals.shrink_to_fit();
        }
        Ok(())
    }
}

/// Enum used as signal input type. It has no variants and so can not be instantiated
pub enum SignalsDontTakeInputSilly {}

/// Installing signals breaks without this. Why? Who fucking knows.
impl From<SignalsDontTakeInputSilly> for RequestResult<SignalsDontTakeInputSilly> {
    fn from(value: SignalsDontTakeInputSilly) -> Self {
        Ok(value)
    }
}

impl<T: Clone> Conduit<Vec<T>, SignalsDontTakeInputSilly> for Weak<Dispatcher<T>> {
    fn output(&self, _: &State) -> RequestResult<Vec<T>> {
        let dispatcher = self
            .upgrade()
            .ok_or_else(|| InternalError("signal no longer exists".into()))?;
        let signals = dispatcher.signals.lock().unwrap();
        // We have to clone because there might be mutliple subscribers
        Ok(signals.signals.clone())
    }

    fn input(&self, _: &mut State, _: SignalsDontTakeInputSilly) -> RequestResult<()> {
        unreachable!();
    }

    fn subscribe(&self, _: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        let dispatcher = self
            .upgrade()
            .ok_or_else(|| InternalError("signal no longer exists".into()))?;
        dispatcher.subscribers.subscribe(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, _: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        let dispatcher = self
            .upgrade()
            .ok_or_else(|| InternalError("signal no longer exists".into()))?;
        dispatcher.subscribers.unsubscribe(subscriber)?;
        Ok(())
    }
}

/// Similar to `Element<T>`, except it allows the game to fire signals.
pub struct Signal<T> {
    dispatcher: Option<Arc<Dispatcher<T>>>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new() -> Self {
        Self { dispatcher: None }
    }

    /// Fire a signal. Requires mut conceptually even though it could be implemented without it.
    pub fn fire(&mut self, data: T) {
        if let Some(dispatcher) = &self.dispatcher {
            let mut signals = dispatcher.signals.lock().unwrap();
            // Only add the dispatcher to the notification queue for the first signal fired
            if signals.signals.is_empty() {
                match signals.notif_queue.get() {
                    Ok(notif_queue) => notif_queue
                        .extend(std::iter::once(
                            Arc::downgrade(&dispatcher) as Weak<dyn Subscriber>
                        )),
                    Err(e) => error!("failed to fire signal: {}", e),
                }
            }
            signals.signals.push(data);
        }
    }

    /// Send notifications to the given subscriber when the inner value changes
    pub fn conduit(
        &mut self,
        notif_queue: &NotifQueue,
    ) -> impl Conduit<Vec<T>, SignalsDontTakeInputSilly> + Clone {
        if self.dispatcher.is_none() {
            self.dispatcher = Some(Arc::new(Dispatcher::new()));
        }
        let dispatcher = self.dispatcher.as_ref().unwrap();
        let mut signals = dispatcher.signals.lock().unwrap();
        signals
            .notif_queue
            .try_init(notif_queue)
            .or_log_error("problem creating signal conduit");
        Arc::downgrade(&dispatcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_notifications(state: &State) {
        let mut buf = Vec::new();
        state.notif_queue.swap_buffer(&mut buf);
        let handler = MockEventHandler::new();
        for notification in &buf {
            notification
                .upgrade()
                .expect("dead subscriber in notification queue")
                .notify(&state, &handler)
                .expect("notify failed");
        }
    }

    fn setup() -> (
        Signal<i32>,
        State,
        impl Conduit<Vec<i32>, SignalsDontTakeInputSilly> + Clone,
    ) {
        let mut signal = Signal::new();
        let state = State::new();
        let conduit = signal.conduit(&state.notif_queue);
        (signal, state, conduit)
    }

    fn checking_subscriber<C>(conduit: &C, expected: Vec<i32>) -> MockSubscriber
    where
        C: Conduit<Vec<i32>, SignalsDontTakeInputSilly> + Clone + 'static,
    {
        let conduit = conduit.clone();
        MockSubscriber::new_with_fn(move |state| {
            assert_eq!(conduit.output(state), Ok(expected.clone()));
        })
    }

    #[test]
    fn can_fire_without_conduit() {
        let mut signal = Signal::new();
        signal.fire(7);
    }

    #[test]
    fn can_fire_with_unsubscribed_conduit() {
        let (mut signal, state, _) = setup();
        signal.fire(7);
        send_notifications(&state);
    }

    #[test]
    fn subscribed_conduit_gets_notified() {
        let (mut signal, state, conduit) = setup();
        let subscriber = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        signal.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn sees_signals_in_notification() {
        let (mut signal, state, conduit) = setup();
        let subscriber = checking_subscriber(&conduit, vec![7, 3, 120]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        signal.fire(7);
        signal.fire(3);
        signal.fire(120);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn can_have_multiple_subscribers() {
        let (mut signal, state, conduit) = setup();
        let subscriber_a = MockSubscriber::new();
        let subscriber_b = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber_a.get()).unwrap();
        conduit.subscribe(&state, &subscriber_b.get()).unwrap();
        signal.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber_a.notify_count(), 1);
        assert_eq!(subscriber_b.notify_count(), 1);
    }

    #[test]
    fn all_subscribers_see_signal() {
        let (mut signal, state, conduit) = setup();
        let subscriber_a = checking_subscriber(&conduit, vec![7]);
        let subscriber_b = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber_a.get()).unwrap();
        conduit.subscribe(&state, &subscriber_b.get()).unwrap();
        signal.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber_a.notify_count(), 1);
        assert_eq!(subscriber_b.notify_count(), 1);
    }

    #[test]
    fn signals_are_cleared_after_notifications_sent_without_subscriber() {
        let (mut signal, state, conduit) = setup();
        signal.fire(120);
        send_notifications(&state);
        signal.fire(22);
        signal.fire(9);
        send_notifications(&state);
        let subscriber = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        signal.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }

    #[test]
    fn signals_are_cleared_after_notifications_sent_with_subscriber() {
        let (mut signal, state, conduit) = setup();
        let subscriber = checking_subscriber(&conduit, vec![7, 12]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        signal.fire(7);
        signal.fire(12);
        send_notifications(&state);
        signal.fire(7);
        signal.fire(12);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 2);
    }

    #[test]
    fn is_not_notified_on_no_signals() {
        let (mut signal, state, conduit) = setup();
        let subscriber = MockSubscriber::new();
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        signal.fire(7);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
        signal.fire(7);
        signal.fire(12);
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 2);
    }

    #[test]
    fn subscribing_late_still_sends_signals() {
        let (mut signal, state, conduit) = setup();
        signal.fire(7);
        let subscriber = checking_subscriber(&conduit, vec![7]);
        conduit.subscribe(&state, &subscriber.get()).unwrap();
        send_notifications(&state);
        assert_eq!(subscriber.notify_count(), 1);
    }
}
