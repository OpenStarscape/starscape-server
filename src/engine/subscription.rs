use super::*;

struct NullSubscriber;

impl Subscriber for NullSubscriber {
    fn notify(&self, _: &State, _: &dyn EventHandler) -> Result<(), Box<dyn Error>> {
        Err("NullSubscriber::notify() should not have been called".into())
    }
}

pub struct Subscription {
    conduit: Box<dyn Conduit<Encodable, Decoded>>,
    is_unsubscribed: bool,
}

/// Type used to remember and unsubscribe from subscriptions
impl Subscription {
    pub fn new(
        state: &State,
        conduit: Box<dyn Conduit<Encodable, Decoded>>,
    ) -> Result<Self, String> {
        let subscriber: Arc<dyn Subscriber> = Arc::new(NullSubscriber);
        conduit.subscribe(state, &subscriber)?;
        Ok(Self {
            conduit,
            is_unsubscribed: false,
        })
    }

    pub fn unsubscribe(mut self, state: &State) -> Result<(), String> {
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
