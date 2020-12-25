use super::*;

/// A chain of conduits composes the interface between properties, actions and events and the state.
/// `O` is the output/get type and `I` is the input/set type
pub trait Conduit<O, I> {
    fn output(&self, state: &State) -> Result<O, String>;
    fn input(&self, state: &mut State, value: I) -> Result<(), String>;
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String>;
    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String>;

    #[must_use]
    fn map_output<F, OuterO>(self, f: F) -> MapOutputConduit<Self, O, I, F>
    where
        Self: Sized,
        F: Fn(O) -> Result<OuterO, String>,
    {
        MapOutputConduit::new(self, f)
    }

    #[must_use]
    fn map_input<F, OuterI>(self, f: F) -> MapInputConduit<Self, O, I, OuterI, F>
    where
        Self: Sized,
        F: Fn(OuterI) -> Result<I, String>,
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
        O: Into<Encodable> + 'static,
        I: 'static,
        Decoded: Into<Result<I, String>>,
    {
        state.install_property(entity, name, self.map_into::<Encodable, Decoded>());
    }

    fn install_event<T>(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        T: Into<Encodable>,
        O: IntoIterator<Item = T> + 'static,
        I: 'static,
        EventsDontTakeInputSilly: Into<Result<I, String>>,
    {
        let conduit = self
            .map_output(|iter| Ok(iter.into_iter().map(Into::into).collect()))
            .map_input(Into::into);
        state.install_event(entity, name, conduit);
    }
}

pub enum ReadOnlyPropSetType {}

impl From<Decoded> for Result<ReadOnlyPropSetType, String> {
    fn from(_value: Decoded) -> Result<ReadOnlyPropSetType, String> {
        Err("read_only_property".to_string())
    }
}

/// Allows for making a conduit clonable
impl<O, I> Conduit<O, I> for Arc<dyn Conduit<O, I>> {
    fn output(&self, state: &State) -> Result<O, String> {
        (**self).output(state)
    }

    fn input(&self, state: &mut State, value: I) -> Result<(), String> {
        (**self).input(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        (**self).subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        (**self).unsubscribe(state, subscriber)
    }
}
