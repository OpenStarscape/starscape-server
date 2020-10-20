use super::*;

pub trait Property {
    fn get_value(&self, state: &State) -> Result<Encodable, String>;
    fn set_value(&self, state: &mut State, value: &Decodable) -> Result<(), String>;
    fn subscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String>;
    fn unsubscribe(&self, state: &State, subscriber: ConnectionKey) -> Result<(), String>;
    fn finalize(&self, state: &State);
}
