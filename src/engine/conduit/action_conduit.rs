use super::*;

/// A conduit that handles a client action
pub struct ActionConduit<T, IFn>
where
    IFn: Fn(&mut State, T) -> RequestResult<()> + 'static,
{
    input_fn: IFn,
    phantom_t: PhantomData<T>,
}

impl<T, IFn> ActionConduit<T, IFn>
where
    IFn: Fn(&mut State, T) -> RequestResult<()> + 'static,
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
    T: Send + Sync,
    IFn: Fn(&mut State, T) -> RequestResult<()> + Send + Sync + 'static,
{
    fn output(&self, _: &State) -> RequestResult<ActionsDontProduceOutputSilly> {
        Err(BadRequest("can not get value from action".to_string()))
    }

    fn input(&self, state: &mut State, value: T) -> RequestResult<()> {
        (self.input_fn)(state, value)
    }

    fn subscribe(&self, _: &State, _: &Arc<dyn Subscriber>) -> RequestResult<()> {
        Err(BadRequest("can not subscribe to action".to_string()))
    }

    fn unsubscribe(&self, _: &State, _: &Weak<dyn Subscriber>) -> RequestResult<()> {
        Err(BadRequest("can not unsubscribe from action".to_string()))
    }
}
