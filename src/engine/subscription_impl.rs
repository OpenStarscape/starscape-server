use super::*;

struct NullSubscriber;

impl Subscriber for NullSubscriber {
    fn notify(&self, _: &State, _: &dyn EventHandler) {}
}

/// This is the type casted to an Any and given to the connection to represent a subscription. It
/// subscribes a null subscriber to the conduit, which causes the conduit to start spewing updates.
/// This works ? for some reason? It's very jank and I don't completely understand it, but
/// refactoring takes significant thought.
pub struct SubscriptionImpl {
    conduit: Box<dyn Conduit<Value, Value>>,
    is_unsubscribed: bool,
}

impl SubscriptionImpl {
    pub fn new(state: &State, conduit: Box<dyn Conduit<Value, Value>>) -> RequestResult<Self> {
        let subscriber: Arc<dyn Subscriber> = Arc::new(NullSubscriber);
        conduit.subscribe(state, &subscriber)?;
        Ok(Self {
            conduit,
            is_unsubscribed: false,
        })
    }
}

impl Subscription for SubscriptionImpl {
    fn finalize(mut self: Box<Self>, handler: &dyn RequestHandler) -> RequestResult<()> {
        let state: &State = (handler.as_ref() as &dyn Any)
            .downcast_ref()
            .ok_or_else(|| {
                RequestError::InternalError(
                    "SubscriptionImpl given RequestHandler that is not a State".into(),
                )
            })?;
        self.is_unsubscribed = true;
        let subscriber: Weak<dyn Subscriber> = Weak::<NullSubscriber>::new();
        self.conduit.unsubscribe(state, &subscriber)
    }
}

impl Drop for SubscriptionImpl {
    fn drop(&mut self) {
        if !self.is_unsubscribed {
            error!("Subscription dropped without being unsubscribed");
        }
    }
}
