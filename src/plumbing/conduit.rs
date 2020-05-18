use crate::server::Encodable;
use crate::state::{PropertyKey, State};

/// The interface between a property and the state
pub trait Conduit {
    fn get_value(&self, state: &State) -> Result<Encodable, String>;
    fn set_value(&self, state: &mut State, value: ()) -> Result<(), String>;
    fn connect(&self, state: &State, property: PropertyKey) -> Result<(), String>;
    fn disconnect(&self, state: &State, property: PropertyKey) -> Result<(), String>;
}
