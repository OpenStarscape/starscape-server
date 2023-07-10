use super::*;

pub struct MapInputConduit<C, O, InnerI, OuterI, F>
where
    C: Conduit<O, InnerI>,
    F: Fn(OuterI) -> (Option<InnerI>, RequestResult<()>),
{
    conduit: C,
    f: F,
    get_pd: PhantomData<O>,
    inner_i_pd: PhantomData<InnerI>,
    outer_i_pd: PhantomData<OuterI>,
}

impl<C, F, O, InnerI, OuterI> MapInputConduit<C, O, InnerI, OuterI, F>
where
    C: Conduit<O, InnerI>,
    F: Fn(OuterI) -> (Option<InnerI>, RequestResult<()>),
{
    pub fn new(conduit: C, f: F) -> Self {
        Self {
            conduit,
            f,
            get_pd: PhantomData,
            inner_i_pd: PhantomData,
            outer_i_pd: PhantomData,
        }
    }
}

impl<C, F, Get, SetInner, SetOuter> Conduit<Get, SetOuter>
    for MapInputConduit<C, Get, SetInner, SetOuter, F>
where
    C: Conduit<Get, SetInner>,
    F: Fn(SetOuter) -> (Option<SetInner>, RequestResult<()>) + Send + Sync,
    Get: Send + Sync,
    SetInner: Send + Sync,
    SetOuter: Send + Sync,
{
    fn output(&self, state: &State) -> RequestResult<Get> {
        self.conduit.output(state)
    }

    fn input(&self, state: &mut State, value: SetOuter) -> RequestResult<()> {
        let (value, result) = (self.f)(value);
        if let Some(value) = value {
            self.conduit.input(state, value)?;
        }
        result
    }
}

impl<C, F, Get, SetInner, SetOuter> Subscribable for MapInputConduit<C, Get, SetInner, SetOuter, F>
where
    C: Conduit<Get, SetInner>,
    F: Fn(SetOuter) -> (Option<SetInner>, RequestResult<()>) + Send + Sync,
    Get: Send + Sync,
    SetInner: Send + Sync,
    SetOuter: Send + Sync,
{
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
