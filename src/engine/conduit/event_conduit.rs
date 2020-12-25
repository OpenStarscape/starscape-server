use super::*;

/// The top-most conduit of an event, automatically created by State. Note that this conduit
/// does not use the given subscriber (it handles dispatching events itself). It uses .subscribe()
/// and .unsubscribe() to know when to start and stop dispatching updates, but it does not care
/// what subscriber is sent.
pub struct EventConduit<C> {
    connection: ConnectionKey,
    entity: EntityKey,
    name: &'static str,
    inner: C,
}

impl<C> EventConduit<C>
where
    C: Conduit<Vec<Encodable>, EventsDontTakeInputSilly> + 'static,
{
    pub fn new(
        connection: ConnectionKey,
        entity: EntityKey,
        name: &'static str,
        inner: C,
    ) -> Box<dyn Conduit<Encodable, Decoded>> {
        Box::new(Arc::new(Self {
            connection,
            entity,
            name,
            inner,
        }))
    }
}

impl<C> Subscriber for EventConduit<C>
where
    C: Conduit<Vec<Encodable>, EventsDontTakeInputSilly> + 'static,
{
    fn notify(
        &self,
        state: &State,
        handler: &dyn OutboundMessageHandler,
    ) -> Result<(), Box<dyn Error>> {
        let events = self.inner.output(state)?;
        for event in events {
            if let Err(e) = handler.event(self.connection, self.entity, self.name, &event) {
                error!("dispatching event: {}", e);
            }
        }
        Ok(())
    }
}

impl<C> Conduit<Encodable, Decoded> for Arc<EventConduit<C>>
where
    C: Conduit<Vec<Encodable>, EventsDontTakeInputSilly> + 'static,
{
    fn output(&self, _: &State) -> Result<Encodable, String> {
        Err("can not get value from event".into())
    }

    fn input(&self, _: &mut State, _: Decoded) -> Result<(), String> {
        Err("can not set value on event".into())
    }

    /// Uses this as a signal to subscribe, but ignores the given subscriber.
    fn subscribe(&self, state: &State, _: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.inner
            .subscribe(state, &(self.clone() as Arc<dyn Subscriber>))
    }

    /// Uses this as a signal to unsubscribe, but ignores the given subscriber.
    fn unsubscribe(&self, state: &State, _: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.inner
            .unsubscribe(state, &(Arc::downgrade(self) as Weak<dyn Subscriber>))
    }
}
