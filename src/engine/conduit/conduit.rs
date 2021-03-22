use super::*;

/// A chain of conduits composes the interface between properties, actions and signals and the state.
/// `O` is the output/get type and `I` is the input/set type
pub trait Conduit<O, I>: Subscribable + Send + Sync {
    fn output(&self, state: &State) -> RequestResult<O>;
    fn input(&self, state: &mut State, value: I) -> RequestResult<()>;

    #[must_use]
    fn map_output<F, OuterO>(self, f: F) -> MapOutputConduit<Self, O, I, F>
    where
        Self: Sized,
        F: Fn(O) -> RequestResult<OuterO>,
    {
        MapOutputConduit::new(self, f)
    }

    #[must_use]
    fn map_input<F, OuterI>(self, f: F) -> MapInputConduit<Self, O, I, OuterI, F>
    where
        Self: Sized,
        F: Fn(OuterI) -> RequestResult<I>,
    {
        MapInputConduit::new(self, f)
    }

    #[must_use]
    fn map_into<ResultGet, ResultSet>(self) -> TryIntoConduit<Self, O, I>
    where
        Self: Sized,
    {
        TryIntoConduit::new(self)
    }

    fn install_property(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        O: Into<Value> + Send + Sync + 'static,
        I: Send + Sync + 'static,
        Value: Into<RequestResult<I>>,
    {
        state.install_property(entity, name, self.map_into::<Value, Value>());
    }

    fn install_signal<T>(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        T: Into<Value>,
        O: IntoIterator<Item = T> + Send + Sync + 'static,
        I: Send + Sync + 'static,
        SignalsDontTakeInputSilly: Into<RequestResult<I>>,
    {
        let conduit = self
            .map_output(|iter| Ok(iter.into_iter().map(Into::into).collect()))
            .map_input(Into::into);
        state.install_signal(entity, name, conduit);
    }

    fn install_action(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        O: Into<ActionsDontProduceOutputSilly> + Send + Sync + 'static,
        I: Send + Sync + 'static,
        Value: Into<RequestResult<I>>,
    {
        state.install_action(
            entity,
            name,
            self.map_into::<ActionsDontProduceOutputSilly, Value>(),
        );
    }
}

pub enum ReadOnlyPropSetType {}

impl From<Value> for RequestResult<ReadOnlyPropSetType> {
    fn from(_value: Value) -> RequestResult<ReadOnlyPropSetType> {
        Err(BadRequest("read only property".into()))
    }
}

/// Allows for making a conduit clonable
impl<O, I> Conduit<O, I> for Arc<dyn Conduit<O, I>> {
    fn output(&self, state: &State) -> RequestResult<O> {
        (**self).output(state)
    }

    fn input(&self, state: &mut State, value: I) -> RequestResult<()> {
        (**self).input(state, value)
    }
}

impl<O, I> Subscribable for Arc<dyn Conduit<O, I>> {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        (**self).subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        (**self).unsubscribe(state, subscriber)
    }
}
