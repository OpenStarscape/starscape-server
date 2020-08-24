use super::*;

pub trait NotificationSink {
    fn notify(&self, state: &State, server: &dyn PropertyUpdateSink) -> Result<(), Box<dyn Error>>;
}

impl dyn NotificationSink {
    /// Weak/Arc::ptr_eq() are broken. See https://github.com/rust-lang/rust/issues/46139. Use this instead
    pub fn thin_ptr(weak: &Weak<dyn NotificationSink>) -> *const () {
        match weak.upgrade() {
            Some(arc) => Arc::as_ptr(&arc) as *const (),
            None => std::ptr::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockNotificationSink;

    impl NotificationSink for MockNotificationSink {
        fn notify(
            &self,
            _state: &State,
            _server: &dyn PropertyUpdateSink,
        ) -> Result<(), Box<dyn Error>> {
            panic!("MockNotificationSink.notify() should not have been called");
        }
    }

    #[test]
    fn thin_ptr_returns_same_for_clones() {
        let sink = Arc::new(MockNotificationSink {});
        let a = Arc::downgrade(&(sink.clone() as Arc<dyn NotificationSink>));
        let b = Arc::downgrade(&sink) as Weak<dyn NotificationSink>;
        assert_eq!(
            NotificationSink::thin_ptr(&a),
            NotificationSink::thin_ptr(&b)
        );
    }

    #[test]
    fn thin_ptr_doesnt_return_null() {
        let sink = Arc::new(MockNotificationSink {});
        let weak = Arc::downgrade(&sink) as Weak<dyn NotificationSink>;
        assert_ne!(NotificationSink::thin_ptr(&weak), std::ptr::null());
    }

    #[test]
    fn thin_ptr_returns_different_for_different_sinks() {
        let sink_a = Arc::new(MockNotificationSink {});
        let sink_b = Arc::new(MockNotificationSink {});
        let a = Arc::downgrade(&sink_a) as Weak<dyn NotificationSink>;
        let b = Arc::downgrade(&sink_b) as Weak<dyn NotificationSink>;
        assert_ne!(
            NotificationSink::thin_ptr(&a),
            NotificationSink::thin_ptr(&b)
        );
    }

    #[test]
    fn thin_ptr_returns_null_for_empty_weak() {
        let weak;
        {
            let sink = Arc::new(MockNotificationSink {});
            weak = Arc::downgrade(&sink) as Weak<dyn NotificationSink>;
        }
        assert!(weak.upgrade().is_none());
        assert_eq!(NotificationSink::thin_ptr(&weak), std::ptr::null());
    }
}
