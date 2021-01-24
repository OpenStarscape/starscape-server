use super::*;

/// An object that can be notified after the state is updated.
pub trait Subscriber {
    fn notify(&self, state: &State, handler: &dyn EventHandler);
}
