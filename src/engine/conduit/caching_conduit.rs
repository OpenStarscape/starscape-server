use super::*;

/// The default property implementation
pub struct CachingConduit<C, T>
where
    C: Conduit<T, T>,
    T: PartialEq + 'static,
{
    weak_self: WeakSelf<CachingConduit<C, T>>,
    cached_value: Mutex<Option<T>>,
    conduit: C,
    subscribers: ConduitSubscriberList,
}

impl<C, T> CachingConduit<C, T>
where
    C: Conduit<T, T> + 'static,
    T: PartialEq + Clone + 'static,
{
    pub fn new(conduit: C) -> Arc<Self> {
        let result = Arc::new(Self {
            weak_self: WeakSelf::new(),
            cached_value: Mutex::new(None),
            conduit,
            subscribers: ConduitSubscriberList::new(),
        });
        result.weak_self.init(&result);
        result
    }
}

impl<C, T> Subscriber for CachingConduit<C, T>
where
    C: Conduit<T, T>,
    T: PartialEq,
{
    fn notify(
        &self,
        state: &State,
        sink: &dyn OutboundMessageHandler,
    ) -> Result<(), Box<dyn Error>> {
        let value = self.conduit.output(state)?;
        let mut cached = self
            .cached_value
            .lock()
            .expect("failed to lock cached Encodable mutex");
        if cached.as_ref() != Some(&value) {
            *cached = Some(value);
            self.subscribers.send_notifications(state, sink);
        }
        Ok(())
    }
}

impl<C, T> Conduit<T, T> for CachingConduit<C, T>
where
    C: Conduit<T, T> + 'static,
    T: PartialEq,
{
    fn output(&self, state: &State) -> Result<T, String> {
        // TODO: use cache if it is up to date
        self.conduit.output(state)
    }

    fn input(&self, state: &mut State, value: T) -> Result<(), String> {
        // TODO: don't set if same as cache
        self.conduit.input(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        if self.subscribers.subscribe(subscriber)?.was_empty {
            if let Err(e) = self.conduit.subscribe(
                state,
                &self.weak_self.get().upgrade().ok_or_else(|| {
                    "self.self_subscriber is null (this should never happen)".to_string()
                })?,
            ) {
                error!("subscribing caching conduit: {}", e);
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        if self.subscribers.unsubscribe(subscriber)?.is_now_empty {
            if let Err(e) = self
                .conduit
                .unsubscribe(state, &(self.weak_self.get() as Weak<dyn Subscriber>))
            {
                error!("unsubscribing caching conduit: {}", e);
            }
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    struct MockConduit {
        value_to_get: Result<i32, String>,
        subscribed: Option<Weak<dyn Subscriber>>,
    }

    impl MockConduit {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                value_to_get: Err("no Encodable yet".to_owned()),
                subscribed: None,
            }))
        }
    }

    impl Conduit<i32, i32> for Rc<RefCell<MockConduit>> {
        fn output(&self, _sate: &State) -> Result<i32, String> {
            self.borrow().value_to_get.clone()
        }

        fn input(&self, _state: &mut State, _value: i32) -> Result<(), String> {
            panic!("unexpected call");
        }

        fn subscribe(
            &self,
            _state: &State,
            subscriber: &Arc<dyn Subscriber>,
        ) -> Result<(), String> {
            assert!(self.borrow().subscribed.is_none());
            self.borrow_mut().subscribed = Some(Arc::downgrade(subscriber));
            Ok(())
        }

        fn unsubscribe(
            &self,
            _state: &State,
            subscriber: &Weak<dyn Subscriber>,
        ) -> Result<(), String> {
            if let Some(s) = &self.borrow().subscribed {
                assert_eq!(s.thin_ptr(), subscriber.thin_ptr());
            } else {
                panic!();
            }
            self.borrow_mut().subscribed = None;
            Ok(())
        }
    }

    fn setup() -> (
        State,
        Arc<CachingConduit<Rc<RefCell<MockConduit>>, i32>>,
        Rc<RefCell<MockConduit>>,
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
        assert!(inner.borrow().subscribed.is_none());
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        assert!(inner.borrow().subscribed.is_some());
    }

    #[test]
    fn subscribes_self_to_inner() {
        let (state, caching, inner, sinks, _) = setup();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        if let Some(subscribed_to) = inner.borrow().subscribed.clone() {
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
        assert!(inner.borrow().subscribed.is_some());
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
        assert!(inner.borrow().subscribed.is_some());
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
        assert!(inner.borrow().subscribed.is_none());
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
        assert!(inner.borrow().subscribed.is_none());
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
        let prop_update_sink = MockOutboundMessageHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(42);
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(mock_sinks[0].notify_count(), 1);
    }

    #[test]
    fn notified_subscribers_when_updated_multiple_times() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockOutboundMessageHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(42);
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        inner.borrow_mut().value_to_get = Ok(69);
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(mock_sinks[0].notify_count(), 2);
    }

    #[test]
    fn does_not_notify_property_update_sink_when_same_data_sent_twice() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockOutboundMessageHandler::new();
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(42);
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(mock_sinks[0].notify_count(), 1);
    }
}
