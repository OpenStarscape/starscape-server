use crate::connection::Value;
use crate::state::State;

/// The interface between a property and the state
pub trait Conduit {
    fn get_value(&self, state: &State) -> Result<Value, String>;
    fn set_value(&self, state: &mut State, value: ()) -> Result<(), String>;
    fn connect(&self, state: &State) -> Result<(), String>;
    fn disconnect(&self, state: &State) -> Result<(), String>;
}
