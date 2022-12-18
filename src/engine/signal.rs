use super::*;

struct PendingSignalEvents<T> {
    /// Holds all the signal events queued up to be send this tick. When the dispatcher is notified
    /// all subscribers registered to the notif_queue are notified to give them a chance to process
    /// the signal events, then all events are dropped.
    signal_events: Vec<T>,
    /// Must be inside the lock becuase it's initialized after construction. Shared handle to the
    /// State's NotifQueue. Our subscribers __do not__ go here. Instead the dispatcher goes here,
    /// which then notifies our subscribers. This is so we only get one notification per tick, and
    /// can flush the pending events at the end of it without future notifications wanting them.
    state_notif_queue: Initializable<NotifQueue>,
}

/// Managing dispatching a signal to a list of subscribers. This is tricky because there can be
/// multiple signal events in a single tick and multiple subscribers and no great way to flush all
/// the events at the end. Instead of putting all subscribers on the State's NotifQueue, we put this
/// dispatcher on the State's NotifQueue no more than once, and manage notifying subscribers
/// ourselves. This way we can know when all subscribers have been notified for this tick and clear
/// pending signal events.
struct Dispatcher<T> {
    /// The signal events that have been queued up and not yet dispatched
    pending: Mutex<PendingSignalEvents<T>>,
    /// Needs to be on a differnt lock than pending so notify() doesn't deadlock
    subscribers: SyncSubscriberList,
}

impl<T> Dispatcher<T> {
    fn new() -> Self {
        Self {
            pending: Mutex::new(PendingSignalEvents {
                signal_events: Vec::new(),
                state_notif_queue: Initializable::new(),
            }),
            subscribers: SyncSubscriberList::new(),
        }
    }
}

impl<T: Send + Sync> Subscriber for Dispatcher<T> {
    fn notify(&self, state: &State, handler: &dyn EventHandler) {
        // this notifies our subscribers,
        self.subscribers.send_notifications(state, handler);
        // now that the notifications have been processed we clear the pending signals
        let mut pending = self.pending.lock().unwrap();
        pending.signal_events.clear();
        // Balence between not letting a very active tick consume a lot of memory indefinitely and
        // not requiring reallocation every time
        if pending.signal_events.capacity() > 10 {
            pending.signal_events.shrink_to_fit();
        }
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

impl<T: Clone + Send + Sync> Conduit<Vec<T>, SignalsDontTakeInputSilly> for Arc<Dispatcher<T>> {
    fn output(&self, _: &State) -> RequestResult<Vec<T>> {
        let pending = self.pending.lock().unwrap();
        // We have to clone because there might be mutliple subscribers
        Ok(pending.signal_events.clone())
    }

    fn input(&self, _: &mut State, _: SignalsDontTakeInputSilly) -> RequestResult<()> {
        unreachable!();
    }
}

impl<T: Clone + Send + Sync> Subscribable for Arc<Dispatcher<T>> {
    fn subscribe(&self, _: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.subscribers.add(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, _: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        self.subscribers.remove(subscriber)?;
        Ok(())
    }
}

/// Similar to `Element<T>`, except it allows the game to fire signals.
pub struct Signal<T> {
    dispatcher: Arc<Dispatcher<T>>,
}

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    pub fn new() -> Self {
        Self {
            dispatcher: Arc::new(Dispatcher::new()),
        }
    }

    /// Fire a signal. Requires mut conceptually even though it could be implemented without it.
    pub fn fire(&mut self, data: T) {
        let mut pending = self.dispatcher.pending.lock().unwrap();
        // Only add the dispatcher to the notification queue for the first signal fired
        if pending.signal_events.is_empty() {
            match pending.state_notif_queue.get() {
                Ok(notif_queue) => notif_queue
                    .extend(std::iter::once(
                        Arc::downgrade(&self.dispatcher) as Weak<dyn Subscriber>
                    )),
                Err(e) => error!("failed to fire signal: {}", e),
            }
        }
        pending.signal_events.push(data);
    }

    /// Send notifications to the given subscriber when the inner value changes
    pub fn conduit(
        &self,
        notif_queue: &NotifQueue,
    ) -> impl Conduit<Vec<T>, SignalsDontTakeInputSilly> + Clone {
        let mut pending = self.dispatcher.pending.lock().unwrap();
        pending
            .state_notif_queue
            .try_init_with_clone(notif_queue)
            .or_log_error("problem creating signal conduit");
        self.dispatcher.clone()
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
                .notify(&state, &handler);
        }
    }

    fn setup() -> (
        Signal<i32>,
        State,
        impl Conduit<Vec<i32>, SignalsDontTakeInputSilly> + Clone,
    ) {
        let signal = Signal::new();
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
