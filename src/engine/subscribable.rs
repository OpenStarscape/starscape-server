use super::*;

/// An object that can be notified after the state is updated.
pub trait Subscribable {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()>;
    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()>;
}
