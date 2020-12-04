use super::*;

/// The interface between a property and the state
pub trait Conduit: CloneConduit {
    fn get_value(&self, state: &State) -> Result<Encodable, String>;
    fn set_value(&self, state: &mut State, value: &Decoded) -> Result<(), String>;
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String>;
    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String>;
}

pub trait CloneConduit {
    fn clone_conduit(&self) -> Box<dyn Conduit>;
}

impl<T: Conduit + Clone + 'static> CloneConduit for T {
    fn clone_conduit(&self) -> Box<dyn Conduit> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Conduit> {
    fn clone(&self) -> Self {
        self.clone_conduit()
    }
}
