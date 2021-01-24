use super::*;

struct NullSubscriber;

impl Subscriber for NullSubscriber {
    fn notify(&self, _: &State, _: &dyn EventHandler) {}
}

pub struct Subscription {
    conduit: Box<dyn Conduit<Value, Value>>,
    is_unsubscribed: bool,
}

/// This is the type casted to an Any and given to the connection to represent a subscription. It
/// subscribes a null subscriber to the conduit, which causes the conduit to start spewing updates.
/// This works ? for some reason? It's very jank and I don't completely understand it, but
/// refactoring takes significant thought.
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
