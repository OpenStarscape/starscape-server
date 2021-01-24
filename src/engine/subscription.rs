use super::*;

struct NullSubscriber;

impl Subscriber for NullSubscriber {
    fn notify(&self, _: &State, _: &dyn EventHandler) {}
}

pub struct Subscription {
    conduit: Box<dyn Conduit<Value, Value>>,
    is_unsubscribed: bool,
}

/// Type used to remember and unsubscribe from subscriptions
/// TODO: wtf is this?
impl Subscription {
    pub fn new(state: &State, conduit: Box<dyn Conduit<Value, Value>>) -> RequestResult<Self> {
        let subscriber: Arc<dyn Subscriber> = Arc::new(NullSubscriber);
        conduit.subscribe(state, &subscriber)?;
        Ok(Self {
            conduit,
            is_unsubscribed: false,
        })
    }

    pub fn unsubscribe(mut self, state: &State) -> RequestResult<()> {
        self.is_unsubscribed = true;
        let subscriber: Weak<dyn Subscriber> = Weak::<NullSubscriber>::new();
        self.conduit.unsubscribe(state, &subscriber)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if !self.is_unsubscribed {
            error!("Subscription dropped without being unsubscribed");
        }
    }
}
