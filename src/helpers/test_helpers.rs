use super::*;

use std::{
    any::Any,
    cell::RefCell,
    sync::mpsc::RecvTimeoutError::{Disconnected, Timeout},
    thread,
    time::Duration,
};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Doesn't work for everything, but can be useful in tests
pub fn attempt_any_to_string(any: &dyn Any) -> String {
    if let Some(s) = any.downcast_ref::<&str>() {
        format!("&str{{ {:?} }}", s)
    } else if let Some(s) = any.downcast_ref::<String>() {
        format!("String{{ {:?} }}", s)
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any + Send>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any + Send + Sync>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else {
        format!("non-string Any value with {:?}", any.type_id())
    }
}

/// Try to run the given function, or panic if it takes too long
/// Originally from https://github.com/rust-lang/rfcs/issues/2798#issuecomment-552949300
pub fn run_with_specific_timeout<T, F>(d: Duration, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (done_tx, done_rx) = channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(()) => match handle.join() {
            Ok(result) => result,
            Err(e) => panic!(
                "thread panicked but channel was not disconnected: {}",
                attempt_any_to_string(&*e)
            ),
        },
        Err(Disconnected) => match handle.join() {
            Ok(_) => panic!("thread did not panic but channel was disconnected"),
            Err(e) => panic!("thread panicked: {}", attempt_any_to_string(&*e)),
        },
        Err(Timeout) => panic!("thread timed out"),
    }
}

/// Try to run the given function, or panic if it takes longer than DEFAULT_TIMEOUT
pub fn run_with_timeout<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    run_with_specific_timeout(DEFAULT_TIMEOUT, f)
}

/// Returns a list of stotmap of the given length, should only be called once per test for each key
/// type (else you'll get duplicate keys)
pub fn mock_keys<T: Key>(number: u32) -> Vec<T> {
    let mut map = slotmap::DenseSlotMap::with_key();
    (0..number).map(|_| map.insert(())).collect()
}

pub struct MockOutboundMessageHandler(
    pub RefCell<Vec<(ConnectionKey, EntityKey, String, Encodable)>>,
);

impl MockOutboundMessageHandler {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }
}

impl OutboundMessageHandler for MockOutboundMessageHandler {
    fn property_update(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        self.0
            .borrow_mut()
            .push((connection, entity, property.to_owned(), value.clone()));
        Ok(())
    }
    fn event(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        property: &str,
        value: &Encodable,
    ) -> Result<(), Box<dyn Error>> {
        self.0
            .borrow_mut()
            .push((connection, entity, property.to_owned(), value.clone()));
        Ok(())
    }
}

struct MockSubscriberInner {
    count: RefCell<u32>,
    f: Box<dyn Fn(&State)>,
}

pub struct MockSubscriber(Arc<MockSubscriberInner>);

impl MockSubscriber {
    pub fn new() -> Self {
        Self::new_with_fn(|_| ())
    }

    pub fn new_terrified() -> Self {
        Self::new_with_fn(|_| panic!("mock subscriber should not have been notified"))
    }

    pub fn new_with_fn<F>(f: F) -> Self
    where
        F: Fn(&State) + 'static,
    {
        Self(Arc::new(MockSubscriberInner {
            count: RefCell::new(0),
            f: Box::new(f),
        }))
    }

    pub fn get(&self) -> Arc<dyn Subscriber> {
        self.0.clone()
    }

    pub fn notify_count(&self) -> u32 {
        *self.0.count.borrow()
    }
}

impl Subscriber for MockSubscriberInner {
    fn notify(&self, state: &State, _: &dyn OutboundMessageHandler) -> Result<(), Box<dyn Error>> {
        *self.count.borrow_mut() += 1;
        (self.f)(state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_keys_all_different() {
        let k: Vec<slotmap::DefaultKey> = mock_keys(3);
        assert_eq!(k.len(), 3);
        assert_ne!(k[0], k[1]);
        assert_ne!(k[0], k[2]);
        assert_ne!(k[1], k[2]);
    }
}

#[cfg(test)]
mod attempt_any_to_string_tests {
    use super::*;

    #[test]
    fn works_on_str() {
        assert_eq!(attempt_any_to_string(&"foo"), "&str{ \"foo\" }");
    }

    #[test]
    fn works_on_string() {
        assert_eq!(
            attempt_any_to_string(&("foo").to_string()),
            "String{ \"foo\" }"
        );
    }

    #[test]
    fn works_on_boxed_any() {
        let boxed_any: Box<dyn Any> = Box::new("foo");
        let boxed_send_any: Box<dyn Any + Send> = Box::new("foo");
        let boxed_send_sync_any: Box<dyn Any + Send + Sync> = Box::new("foo");
        let expected = "Box{ &str{ \"foo\" } }";
        assert_eq!(attempt_any_to_string(&boxed_any), expected);
        assert_eq!(attempt_any_to_string(&boxed_send_any), expected);
        assert_eq!(attempt_any_to_string(&boxed_send_sync_any), expected);
    }
}

#[cfg(test)]
mod run_with_timeout_tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn runs_fn() {
        let value_a = Arc::new(Mutex::new(7));
        let value_b = value_a.clone();
        run_with_timeout(move || {
            thread::sleep(Duration::from_millis(20));
            *value_a.lock().expect("failed to lock mutex") = 5;
            thread::sleep(Duration::from_millis(20));
        });
        assert_eq!(*value_b.lock().expect("failed to lock mutex"), 5);
    }

    #[test]
    fn returns_value() {
        let result = run_with_timeout(move || {
            thread::sleep(Duration::from_millis(20));
            12
        });
        assert_eq!(result, 12);
    }

    #[test]
    fn does_not_time_out_if_quick() {
        run_with_specific_timeout(Duration::from_millis(50), move || {
            thread::sleep(Duration::from_millis(10));
        });
    }

    #[test]
    #[should_panic(expected = "timed out")]
    fn times_out() {
        run_with_specific_timeout(Duration::from_millis(50), move || {
            thread::sleep(Duration::from_secs(5));
        });
        unreachable!();
    }

    #[test]
    #[should_panic(expected = "this is fine")]
    fn shows_str_panic_message() {
        run_with_timeout(move || {
            panic!("this is fine");
        });
        unreachable!();
    }
}
