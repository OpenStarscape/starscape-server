use crate::connection::Value;
use crate::state::{PropertyKey, State};

/// A fetcher interfaces with a property somewhere in the state
pub trait Conduit {
    fn get_value(&self, state: &State) -> Result<Value, String>;
    fn set_value(&self, state: &mut State, value: ()) -> Result<(), String>;
    fn connect(&self, state: &State, property: PropertyKey) -> Result<(), String>;
    fn disconnect(&self, state: &State, property: PropertyKey) -> Result<(), String>;
}
