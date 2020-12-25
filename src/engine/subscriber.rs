use super::*;

/// An object that can be notified after the state is updated.
/// TODO: why does this return a result? Can the caller be expected to do anything useful with this
/// except log it as an error? Should probably return nothing.
pub trait Subscriber {
    fn notify(
        &self,
        state: &State,
        handler: &dyn OutboundMessageHandler,
    ) -> Result<(), Box<dyn Error>>;
}
