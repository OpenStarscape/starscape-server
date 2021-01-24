use super::*;

/// The top-most conduit of a property, automatically created by State. Note that this conduit
/// does not use the given subscriber (it handles dispatching updates itself). It uses .subscribe()
/// and .unsubscribe() to know when to start and stop dispatching updates, but it does not care
/// what subscriber is sent.
pub struct PropertyConduit<C> {
    connection: ConnectionKey,
    entity: EntityKey,
    name: &'static str,
    inner: C,
}

impl<C> PropertyConduit<C>
where
    C: Conduit<Value, Value> + 'static,
{
    pub fn new(
        connection: ConnectionKey,
        entity: EntityKey,
        name: &'static str,
        inner: C,
    ) -> Box<dyn Conduit<Value, Value>> {
        Box::new(Arc::new(Self {
            connection,
            entity,
            name,
            inner,
        }))
    }
}

impl<C> Subscriber for PropertyConduit<C>
where
    C: Conduit<Value, Value> + 'static,
{
    fn notify(&self, state: &State, handler: &dyn EventHandler) {
        let value = match self.inner.output(state) {
            Ok(value) => value,
            Err(e) => {
                error!("handling property update: {}", e);
                return;
            }
        };
        handler.event(
            self.connection,
            Event::update(self.entity, self.name.to_string(), value),
        );
    }
}

impl<C> Conduit<Value, Value> for Arc<PropertyConduit<C>>
where
    C: Conduit<Value, Value> + 'static,
{
    fn output(&self, state: &State) -> RequestResult<Value> {
        self.inner.output(state)
    }

    fn input(&self, state: &mut State, value: Value) -> RequestResult<()> {
        self.inner.input(state, value)
    }

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
