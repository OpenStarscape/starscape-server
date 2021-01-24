use super::*;

/// The top-most conduit of a signal, automatically created by State. Note that this conduit
/// does not use the given subscriber (it handles dispatching signals itself). It uses .subscribe()
/// and .unsubscribe() to know when to start and stop dispatching updates, but it does not care
/// what subscriber is sent.
pub struct SignalConduit<C> {
    connection: ConnectionKey,
    entity: EntityKey,
    name: &'static str,
    inner: C,
}

impl<C> SignalConduit<C>
where
    C: Conduit<Vec<Value>, SignalsDontTakeInputSilly> + 'static,
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

impl<C> Subscriber for SignalConduit<C>
where
    C: Conduit<Vec<Value>, SignalsDontTakeInputSilly> + 'static,
{
    fn notify(&self, state: &State, handler: &dyn EventHandler) -> Result<(), Box<dyn Error>> {
        let values = self.inner.output(state)?;
        for value in values {
            handler.event(
                self.connection,
                Event::signal(self.entity, self.name.to_string(), value),
            );
        }
        Ok(())
    }
}

impl<C> Conduit<Value, Value> for Arc<SignalConduit<C>>
where
    C: Conduit<Vec<Value>, SignalsDontTakeInputSilly> + 'static,
{
    fn output(&self, _: &State) -> RequestResult<Value> {
        Err(BadRequest("can not get value from signal".into()))
    }

    fn input(&self, _: &mut State, _: Value) -> RequestResult<()> {
        Err(BadRequest("signals do not take input".into()))
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
