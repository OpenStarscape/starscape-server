use super::*;

/// A conduit that handles a client action
pub struct ActionConduit<T, IFn>
where
    IFn: Fn(&mut State, T) -> Result<(), String> + 'static,
{
    input_fn: IFn,
    phantom_t: PhantomData<T>,
}

impl<T, IFn> ActionConduit<T, IFn>
where
    IFn: Fn(&mut State, T) -> Result<(), String> + 'static,
{
    #[must_use]
    pub fn new(input_fn: IFn) -> Self {
        Self {
            input_fn,
            phantom_t: PhantomData,
        }
    }
}

pub enum ActionsDontProduceOutputSilly {}

impl<T, IFn> Conduit<ActionsDontProduceOutputSilly, T> for ActionConduit<T, IFn>
where
    IFn: Fn(&mut State, T) -> Result<(), String> + 'static,
{
    fn output(&self, _: &State) -> Result<ActionsDontProduceOutputSilly, String> {
        Err("can not get value from action".into())
    }

    fn input(&self, state: &mut State, value: T) -> Result<(), String> {
        (self.input_fn)(state, value)
    }

    fn subscribe(&self, _: &State, _: &Arc<dyn Subscriber>) -> Result<(), String> {
        Err("can not subscribe to action".into())
    }

    fn unsubscribe(&self, _: &State, _: &Weak<dyn Subscriber>) -> Result<(), String> {
        Err("can not unsubscribe from action".into())
    }
}
