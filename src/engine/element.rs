use super::*;

struct Subscribers {
    list: SubscriberList,
    notif_queue: Initializable<NotifQueue>,
}

/// An atomic unit of state. An element can be subscribed to, in which case it will notify the
/// subscriber when it is changed. These notifications are __not__ dispatched immediately. Instead,
/// they are queued and processed later in the main game loop.
pub struct Element<T> {
    inner: T,
    subscribers: Mutex<Subscribers>,
    has_subscribers: AtomicBool,
}

impl<T> Default for Element<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Element<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            subscribers: Mutex::new(Subscribers {
                list: SubscriberList::new(),
                notif_queue: Initializable::new(),
            }),
            has_subscribers: AtomicBool::new(false),
        }
    }

    fn queue_notifications(&mut self) {
        if self.has_subscribers.load(SeqCst) {
            let lock = self.subscribers.lock().expect("failed to lock subscribers");
            if !lock.list.0.is_empty() {
                match lock.notif_queue.get() {
                    Ok(notif_queue) => notif_queue.extend(
                        lock.list
                            .0
                            .iter()
                            .map(|(_ptr, subscriber)| subscriber.clone()),
                    ),
                    Err(e) => error!("failed to queue notifications: {}", e),
                }
            }
        }
    }

    /// Prefer set() where possible. That can save work when value is unchanged.
    pub fn get_mut(&mut self) -> &mut T {
        self.queue_notifications();
        &mut self.inner
    }

    /// This is useful, for example, when iterating over a slotmap and modifying elements,
    /// but not adding or removing them
    #[allow(dead_code)]
    pub fn get_mut_without_notifying_of_change(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Subscribable for Element<T> {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.has_subscribers.store(true, SeqCst);
        let mut lock = self.subscribers.lock().unwrap();
        lock.notif_queue
            .try_init_with_clone(&state.notif_queue)
            .map_err(|e| InternalError(format!("failed to subscribe to element: {}", e)))?;
        lock.list.add(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, _: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        let report = self
            .subscribers
            .lock()
            .expect("failed to lock subscribers")
            .list
            .remove(subscriber)?;
        if report.is_now_empty {
            self.has_subscribers.store(false, SeqCst);
        }
        Ok(())
    }
}

impl<T: PartialEq> Element<T> {
    pub fn set(&mut self, value: T) {
        if self.inner != value {
            self.inner = value;
            self.queue_notifications();
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

    fn setup() -> (Element<i64>, State, Arc<dyn Subscriber>) {
        let store = Element::new(7);
        let subscriber = MockSubscriber::new_terrified().get();
        let state = State::new();
        store
            .subscribe(&state, &subscriber)
            .expect("failed to subscribe");
        (store, state, subscriber)
    }

    #[test]
    fn queues_notifications_when_set() {
        let (mut store, state, _) = setup();
        store.set(5);
        assert_eq!(state.notif_queue.len(), 1);
    }

    #[test]
    fn queues_notifications_when_value_mut_accessed() {
        let (mut store, state, _) = setup();
        store.get_mut();
        assert_eq!(state.notif_queue.len(), 1);
    }

    #[test]
    fn does_not_queue_notifications_on_get_mut_without_notifying_of_change() {
        let (mut store, state, _) = setup();
        store.get_mut_without_notifying_of_change();
        assert_eq!(state.notif_queue.len(), 0);
    }

    #[test]
    fn does_not_send_notification_when_set_to_same_value() {
        let (mut store, state, _) = setup();
        store.set(7);
        assert_eq!(state.notif_queue.len(), 0);
    }

    #[test]
    fn unsubscribing_stops_notifications() {
        let (mut store, state, subscriber) = setup();
        // State::new() is ok only because the function doesn't use it, use the real state if this breaks
        store
            .unsubscribe(&State::new(), &Arc::downgrade(&subscriber))
            .expect("unsubbing failed");
        store.set(5);
        assert_eq!(state.notif_queue.len(), 0);
    }
}
