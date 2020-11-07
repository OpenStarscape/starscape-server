use super::*;

/// An object that can be notified after the state is updated
pub trait Subscriber {
    fn notify(&self, state: &State, server: &dyn PropertyUpdateSink) -> Result<(), Box<dyn Error>>;
}

impl dyn Subscriber {
    /// Weak/Arc::ptr_eq() are broken. See https://github.com/rust-lang/rust/issues/46139. Use this instead
    pub fn thin_ptr(weak: &Weak<dyn Subscriber>) -> *const () {
        match weak.upgrade() {
            Some(arc) => Arc::as_ptr(&arc) as *const (),
            None => std::ptr::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSubscriber;

    impl Subscriber for MockSubscriber {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn PropertyUpdateSink,
        ) -> Result<(), Box<dyn Error>> {
            panic!("MockSubscriber.notify() should not have been called");
        }
    }

    #[test]
    fn thin_ptr_returns_same_for_clones() {
        let sink = Arc::new(MockSubscriber {});
        let a = Arc::downgrade(&(sink.clone() as Arc<dyn Subscriber>));
        let b = Arc::downgrade(&sink) as Weak<dyn Subscriber>;
        assert_eq!(Subscriber::thin_ptr(&a), Subscriber::thin_ptr(&b));
    }

    #[test]
    fn thin_ptr_doesnt_return_null() {
        let sink = Arc::new(MockSubscriber {});
        let weak = Arc::downgrade(&sink) as Weak<dyn Subscriber>;
        assert_ne!(Subscriber::thin_ptr(&weak), std::ptr::null());
    }

    #[test]
    fn thin_ptr_returns_different_for_different_sinks() {
        let sink_a = Arc::new(MockSubscriber {});
        let sink_b = Arc::new(MockSubscriber {});
        let a = Arc::downgrade(&sink_a) as Weak<dyn Subscriber>;
        let b = Arc::downgrade(&sink_b) as Weak<dyn Subscriber>;
        assert_ne!(Subscriber::thin_ptr(&a), Subscriber::thin_ptr(&b));
    }

    #[test]
    fn thin_ptr_returns_null_for_empty_weak() {
        let weak;
        {
            let sink = Arc::new(MockSubscriber {});
            weak = Arc::downgrade(&sink) as Weak<dyn Subscriber>;
        }
        assert!(weak.upgrade().is_none());
        assert_eq!(Subscriber::thin_ptr(&weak), std::ptr::null());
    }
}
