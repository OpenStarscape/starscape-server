use super::*;

/// The default property implementation
pub struct CachingConduit<C, T>
where
    C: Conduit<T, T>,
    T: PartialEq + Send + Sync + 'static,
{
    weak_self: WeakSelf<CachingConduit<C, T>>,
    cached_value: Mutex<Option<T>>,
    conduit: C,
    subscribers: SyncSubscriberList,
}

impl<C, T> CachingConduit<C, T>
where
    C: Conduit<T, T> + 'static,
    T: PartialEq + Clone + Send + Sync + 'static,
{
    pub fn new(conduit: C) -> Arc<Self> {
        let result = Arc::new(Self {
            weak_self: WeakSelf::new(),
            cached_value: Mutex::new(None),
            conduit,
            subscribers: SyncSubscriberList::new(),
        });
        result.weak_self.init(&result);
        result
    }
}

impl<C, T> Subscriber for CachingConduit<C, T>
where
    C: Conduit<T, T>,
    T: PartialEq + Send + Sync,
{
    fn notify(&self, state: &State, sink: &dyn EventHandler) {
        let value = match self.conduit.output(state) {
            Ok(value) => value,
            Err(e) => {
                error!("getting value in CachingConduit: {}", e);
                return;
            }
        };
        let mut cached = self
            .cached_value
            .lock()
            .expect("failed to lock cached value mutex");
        if cached.as_ref() != Some(&value) {
            *cached = Some(value);
            self.subscribers.send_notifications(state, sink);
        }
    }
}

impl<C, T> Conduit<T, T> for Arc<CachingConduit<C, T>>
where
    C: Conduit<T, T> + 'static,
    T: PartialEq + Send + Sync,
{
    fn output(&self, state: &State) -> RequestResult<T> {
        // TODO: use cache if it is up to date
        self.conduit.output(state)
    }

    fn input(&self, state: &mut State, value: T) -> RequestResult<()> {
        // TODO: don't set if same as cache
        self.conduit.input(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        if self.subscribers.subscribe(subscriber)?.was_empty {
            let weak_self: Arc<dyn Subscriber> = self
                .weak_self
                .get()
                .upgrade()
                .ok_or_else(|| InternalError("CachingConduit::weak_self is null".into()))?;
            self.conduit
                .subscribe(state, &weak_self)
                .map_err(|e| InternalError(format!("subscribing caching conduit: {}", e)))?;
        }
        Ok(())
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        if self.subscribers.unsubscribe(subscriber)?.is_now_empty {
            self.conduit
                .unsubscribe(state, &(self.weak_self.get() as Weak<dyn Subscriber>))
                .map_err(|e| InternalError(format!("unsubscribing caching conduit: {}", e)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;

    struct MockConduit {
        value_to_get: RequestResult<i32>,
        subscribed: Option<Weak<dyn Subscriber>>,
    }

    impl MockConduit {
        fn new() -> Arc<Mutex<Self>> {
            Arc::new(Mutex::new(Self {
                value_to_get: Err(InternalError("no Value yet".to_owned())),
                subscribed: None,
            }))
        }
    }

    impl Conduit<i32, i32> for Arc<Mutex<MockConduit>> {
        fn output(&self, _sate: &State) -> RequestResult<i32> {
            self.lock().unwrap().value_to_get.clone()
        }

        fn input(&self, _state: &mut State, _value: i32) -> RequestResult<()> {
            panic!("unexpected call");
        }

        fn subscribe(&self, _state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
            assert!(self.lock().unwrap().subscribed.is_none());
            self.lock().unwrap().subscribed = Some(Arc::downgrade(subscriber));
            Ok(())
        }

        fn unsubscribe(
            &self,
            _state: &State,
            subscriber: &Weak<dyn Subscriber>,
        ) -> RequestResult<()> {
            if let Some(s) = &self.lock().unwrap().subscribed {
                assert_eq!(s.thin_ptr(), subscriber.thin_ptr());
            } else {
                panic!();
            }
            self.lock().unwrap().subscribed = None;
            Ok(())
        }
    }

    fn setup() -> (
        State,
        Arc<CachingConduit<Arc<Mutex<MockConduit>>, i32>>,
        Arc<Mutex<MockConduit>>,
        Vec<Arc<dyn Subscriber>>,
        Vec<MockSubscriber>,
    ) {
        let mock_subscribers: Vec<MockSubscriber> = (0..3).map(|_| MockSubscriber::new()).collect();
        let inner = MockConduit::new();
        let caching = CachingConduit::new(inner.clone());
        (
            State::new(),
            caching,
            inner,
            mock_subscribers.iter().map(|s| s.get()).collect(),
            mock_subscribers,
        )
    }

    #[test]
    fn first_subscription_connects_conduit() {
        let (state, caching, inner, sinks, _) = setup();
        assert!(inner.lock().unwrap().subscribed.is_none());
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        assert!(inner.lock().unwrap().subscribed.is_some());
    }

    #[test]
    fn subscribes_self_to_inner() {
        let (state, caching, inner, sinks, _) = setup();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        if let Some(subscribed_to) = inner.lock().unwrap().subscribed.clone() {
            let subscribed_to = subscribed_to.thin_ptr();
            let caching = caching.thin_ptr();
            let sink = sinks[0].thin_ptr();
            assert_ne!(subscribed_to, sink);
            assert_eq!(subscribed_to, caching);
        } else {
            panic!("not subscribed");
        };
    }

    #[test]
    fn subsequent_subscriptions_do_not_connect_conduit() {
        let (state, caching, inner, sinks, _) = setup();
        for sink in sinks {
            // mock conduit should panic if subscribed multiple times
            caching
                .subscribe(&state, &sink)
                .expect("failed to subscribe");
        }
        assert!(inner.lock().unwrap().subscribed.is_some());
    }

    #[test]
    fn does_not_disconnect_on_first_unsubscribe() {
        let (state, caching, inner, sinks, _) = setup();
        for sink in &sinks {
            // mock conduit should panic if subscribed multiple times
            caching
                .subscribe(&state, sink)
                .expect("failed to subscribe");
        }
        caching
            .unsubscribe(&state, &Arc::downgrade(&sinks[0]))
            .expect("failed to unsubscribe");
        assert!(inner.lock().unwrap().subscribed.is_some());
    }

    #[test]
    fn removing_only_subscription_disconnects_conduit() {
        let (state, caching, inner, sinks, _) = setup();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        caching
            .unsubscribe(&state, &Arc::downgrade(&sinks[0]))
            .expect("failed to unsubscribe");
        assert!(inner.lock().unwrap().subscribed.is_none());
    }

    #[test]
    fn removing_all_subscriptions_disconnects_conduit() {
        let (state, caching, inner, sinks, _) = setup();
        for sink in &sinks {
            // mock conduit should panic if subscribed multiple times
            caching
                .subscribe(&state, sink)
                .expect("failed to subscribe");
        }
        for sink in &sinks {
            // mock conduit should panic if unsubscribed multiple times
            caching
                .unsubscribe(&state, &Arc::downgrade(sink))
                .expect("failed to subscribe");
        }
        assert!(inner.lock().unwrap().subscribed.is_none());
    }

    #[test]
    fn single_connection_subscribing_twice_errors() {
        let (state, caching, _, sinks, _) = setup();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        assert!(caching.subscribe(&state, &sinks[0]).is_err());
    }

    #[test]
    fn unsubscribing_with_connection_not_subscribed_errors() {
        let (state, caching, _, sinks, _) = setup();
        assert!(caching
            .unsubscribe(&state, &Arc::downgrade(&sinks[0]))
            .is_err());
    }

    #[test]
    fn notifies_subscribers_when_updated() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockEventHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.lock().unwrap().value_to_get = Ok(42);
        caching.notify(&state, &prop_update_sink);
        assert_eq!(mock_sinks[0].notify_count(), 1);
    }

    #[test]
    fn notified_subscribers_when_updated_multiple_times() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockEventHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.lock().unwrap().value_to_get = Ok(42);
        caching.notify(&state, &prop_update_sink);
        inner.lock().unwrap().value_to_get = Ok(69);
        caching.notify(&state, &prop_update_sink);
        assert_eq!(mock_sinks[0].notify_count(), 2);
    }

    #[test]
    fn does_not_notify_property_update_sink_when_same_data_sent_twice() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockEventHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.lock().unwrap().value_to_get = Ok(42);
        caching.notify(&state, &prop_update_sink);
        caching.notify(&state, &prop_update_sink);
        assert_eq!(mock_sinks[0].notify_count(), 1);
    }
}
