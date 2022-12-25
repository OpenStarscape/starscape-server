use super::*;

/// Sends a connection messages about when an entity is destroyed
pub struct DestructionConduit<C> {
    connection: ConnectionKey,
    object: GenericId,
    inner: C,
}

impl<C> DestructionConduit<C>
where
    C: Conduit<Vec<()>, SignalsDontTakeInputSilly> + 'static,
{
    pub fn new(
        connection: ConnectionKey,
        object: GenericId,
        inner: C,
    ) -> Box<dyn Conduit<Value, Value>> {
        Box::new(Arc::new(Self {
            connection,
            object,
            inner,
        }))
    }
}

impl<C> Subscriber for DestructionConduit<C>
where
    C: Conduit<Vec<()>, SignalsDontTakeInputSilly> + 'static,
{
    fn notify(&self, state: &State, handler: &dyn EventHandler) {
        if let Ok(vec) = self.inner.output(state) {
            if !vec.is_empty() {
                handler.event(state, self.connection, Event::Destroyed(self.object));
            }
        }
    }
}

impl<C> Conduit<Value, Value> for Arc<DestructionConduit<C>>
where
    C: Conduit<Vec<()>, SignalsDontTakeInputSilly> + 'static,
{
    fn output(&self, _: &State) -> RequestResult<Value> {
        Err(BadRequest(
            "can not get value from destruction conduit".into(),
        ))
    }

    fn input(&self, _: &mut State, _: Value) -> RequestResult<()> {
        Err(BadRequest("destruction conduits do not take input".into()))
    }
}
impl<C> Subscribable for Arc<DestructionConduit<C>>
where
    C: Conduit<Vec<()>, SignalsDontTakeInputSilly> + 'static,
{
    /// Uses this as a signal to subscribe, but ignores the given subscriber.
    fn subscribe(&self, state: &State, _: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.inner
            .subscribe(state, &(self.clone() as Arc<dyn Subscriber>))
    }

    /// Uses this as a signal to unsubscribe, but ignores the given subscriber.
    fn unsubscribe(&self, state: &State, _: &Weak<dyn Subscriber>) -> RequestResult<()> {
        self.inner
            .unsubscribe(state, &(Arc::downgrade(self) as Weak<dyn Subscriber>))
    }
}
