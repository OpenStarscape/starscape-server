use std::error::Error;
use std::sync::Mutex;

use super::*;
use crate::{
    server::{Encodable, PropertyUpdateSink},
    state::State,
};

/// The default property implementation
pub struct CachingConduit {
    cached_value: Mutex<Encodable>,
    conduit: Box<dyn Conduit>,
    subscribers: NotificationSource,
}

impl CachingConduit {
    pub fn new(conduit: Box<dyn Conduit>) -> Arc<Self> {
        Arc::new(Self {
            cached_value: Mutex::new(Encodable::Null),
            conduit,
            subscribers: NotificationSource::new(),
        })
    }
}

impl NotificationSink for CachingConduit {
    fn notify(&self, state: &State, sink: &dyn PropertyUpdateSink) -> Result<(), Box<dyn Error>> {
        let value = self.conduit.get_value(state)?;
        let mut cached = self
            .cached_value
            .lock()
            .expect("Failed to lock cached Encodable mutex");
        if *cached != value {
            *cached = value;
            self.subscribers.send_notifications(state, sink);
        }
        Ok(())
    }
}

impl Conduit for Arc<CachingConduit> {
    fn get_value(&self, _state: &State) -> Result<Encodable, String> {
        Ok(self
            .cached_value
            .lock()
            .expect("Failed to lock mutex")
            .clone())
    }

    fn set_value(&self, state: &mut State, value: ()) -> Result<(), String> {
        self.conduit.set_value(state, value)
    }

    fn subscribe(
        &self,
        state: &State,
        subscriber: &Arc<dyn NotificationSink>,
    ) -> Result<(), String> {
        if self.subscribers.subscribe(subscriber)? {
            self.conduit
                .subscribe(state, &(self.clone() as Arc<dyn NotificationSink>))
        } else {
            Ok(())
        }
    }

    fn unsubscribe(
        &self,
        state: &State,
        subscriber: &Weak<dyn NotificationSink>,
    ) -> Result<(), String> {
        if self.subscribers.unsubscribe(subscriber)? {
            self.conduit.unsubscribe(
                state,
                &Arc::downgrade(&(self.clone() as Arc<dyn NotificationSink>)),
            )
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use crate::server::ConnectionKey;
    use std::{cell::RefCell, rc::Rc};

    struct MockNotificationSink(RefCell<u32>);

    impl NotificationSink for MockNotificationSink {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn PropertyUpdateSink,
        ) -> Result<(), Box<dyn Error>> {
            *self.0.borrow_mut() += 1;
            Ok(())
        }
    }

    struct MockPropertyUpdateSink;

    impl PropertyUpdateSink for MockPropertyUpdateSink {
        fn property_changed(
            &self,
            _connection: ConnectionKey,
            _entity: EntityKey,
            _property: &str,
            _value: &Encodable,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    struct MockConduit {
        value_to_get: Result<Encodable, String>,
        subscribed: Option<Weak<dyn NotificationSink>>,
    }

    impl MockConduit {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                value_to_get: Err("No Encodable yet".to_owned()),
                subscribed: None,
            }))
        }
    }

    impl Conduit for Rc<RefCell<MockConduit>> {
        fn get_value(&self, _sate: &State) -> Result<Encodable, String> {
            self.borrow().value_to_get.clone()
        }

        fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
            panic!("Unexpected call");
        }

        fn subscribe(
            &self,
            _state: &State,
            subscriber: &Arc<dyn NotificationSink>,
        ) -> Result<(), String> {
            assert!(self.borrow().subscribed.is_none());
            self.borrow_mut().subscribed = Some(Arc::downgrade(subscriber));
            Ok(())
        }

        fn unsubscribe(
            &self,
            _state: &State,
            subscriber: &Weak<dyn NotificationSink>,
        ) -> Result<(), String> {
            if let Some(s) = &self.borrow().subscribed {
                assert_eq!(
                    NotificationSink::thin_ptr(s),
                    NotificationSink::thin_ptr(subscriber)
                );
            } else {
                panic!();
            }
            self.borrow_mut().subscribed = None;
            Ok(())
        }
    }

    fn setup() -> (
        State,
        Arc<CachingConduit>,
        Rc<RefCell<MockConduit>>,
        Vec<Arc<dyn NotificationSink>>,
        Vec<Arc<MockNotificationSink>>,
    ) {
        let mock_sinks: Vec<Arc<MockNotificationSink>> = (0..3)
            .map(|_| Arc::new(MockNotificationSink(RefCell::new(0))))
            .collect();
        let inner = MockConduit::new();
        let caching = CachingConduit::new(Box::new(inner.clone()));
        (
            State::new(),
            caching,
            inner,
            mock_sinks
                .iter()
                .map(|sink| sink.clone() as Arc<dyn NotificationSink>)
                .collect(),
            mock_sinks,
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
        println!("Caching conduit: {:?}", Arc::as_ptr(&caching));
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        if let Some(subscribed_to) = inner.borrow().subscribed.clone() {
            let subscribed_to = NotificationSink::thin_ptr(&subscribed_to);
            let caching = NotificationSink::thin_ptr(
                &(Arc::downgrade(&caching) as Weak<dyn NotificationSink>),
            );
            let sink = NotificationSink::thin_ptr(&Arc::downgrade(&sinks[0]));
            assert_ne!(subscribed_to, sink);
            assert_eq!(subscribed_to, caching);
        } else {
            panic!("Not subscribed");
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
        let prop_update_sink = MockPropertyUpdateSink;
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(Encodable::Integer(42));
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(*mock_sinks[0].0.borrow(), 1);
    }

    #[test]
    fn notified_subscribers_when_updated_multiple_times() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockPropertyUpdateSink;
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(Encodable::Integer(42));
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        inner.borrow_mut().value_to_get = Ok(Encodable::Integer(69));
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(*mock_sinks[0].0.borrow(), 2);
    }

    #[test]
    fn does_not_notify_property_update_sink_when_same_data_sent_twice() {
        let (state, caching, inner, sinks, mock_sinks) = setup();
        let prop_update_sink = MockPropertyUpdateSink;
        caching
            .subscribe(&state, &sinks[0])
            .expect("failed to subscribe");
        inner.borrow_mut().value_to_get = Ok(Encodable::Integer(42));
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        caching
            .notify(&state, &prop_update_sink)
            .expect("failed to send updates");
        assert_eq!(*mock_sinks[0].0.borrow(), 1);
    }
}
