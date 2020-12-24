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

    fn install(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        O: Into<Encodable> + 'static,
        I: 'static,
        Decoded: Into<Result<I, String>>,
    {
        state.install_property(entity, name, self.map_into::<Encodable, Decoded>());
    }
}

pub enum ReadOnlyPropSetType {}

impl From<Decoded> for Result<ReadOnlyPropSetType, String> {
    fn from(_value: Decoded) -> Result<ReadOnlyPropSetType, String> {
        Err("read_only_property".to_string())
    }
}
