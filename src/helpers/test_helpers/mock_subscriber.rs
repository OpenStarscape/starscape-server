use super::*;

struct MockSubscriberInner {
    count: Mutex<u32>,
    f: Box<dyn Fn(&State) + Send + Sync>,
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
        F: Fn(&State) + Send + Sync + 'static,
    {
        Self(Arc::new(MockSubscriberInner {
            count: Mutex::new(0),
            f: Box::new(f),
        }))
    }

    pub fn get(&self) -> Arc<dyn Subscriber> {
        self.0.clone()
    }

    pub fn notify_count(&self) -> u32 {
        *self.0.count.lock().unwrap()
    }
}

impl Subscriber for MockSubscriberInner {
    fn notify(&self, state: &State, _: &dyn EventHandler) {
        *self.count.lock().unwrap() += 1;
        (self.f)(state);
    }
}
