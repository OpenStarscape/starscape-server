use super::*;

/// The interface between a property and the state
/// G is the type returned by get_value() and S is the type send to set_value()
pub trait Conduit<Get, Set> {
    fn get_value(&self, state: &State) -> Result<Get, String>;
    fn set_value(&self, state: &mut State, value: Set) -> Result<(), String>;
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String>;
    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String>;

    #[must_use]
    fn map_get<F, GetOuter>(self, f: F) -> MapGetConduit<Self, Get, Set, F>
    where
        Self: Sized,
        F: Fn(Get) -> Result<GetOuter, String>,
    {
        MapGetConduit::new(self, f)
    }

    #[must_use]
    fn map_set<F, SetOuter>(self, f: F) -> MapSetConduit<Self, Get, Set, SetOuter, F>
    where
        Self: Sized,
        F: Fn(SetOuter) -> Result<Set, String>,
    {
        MapSetConduit::new(self, f)
    }

    #[must_use]
    fn map_into<ResultGet, ResultSet>(self) -> TryIntoConduit<Self, Get, Set>
    where
        Self: Sized,
    {
        TryIntoConduit::new(self)
    }

    fn install(self, state: &mut State, entity: EntityKey, name: &'static str)
    where
        Self: Sized + 'static,
        Get: Into<Encodable> + 'static,
        Set: 'static,
        Decoded: Into<Result<Set, String>>,
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
