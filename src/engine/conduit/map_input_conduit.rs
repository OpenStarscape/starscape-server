use super::*;

pub struct MapInputConduit<C, O, InnerI, OuterI, F>
where
    C: Conduit<O, InnerI>,
    F: Fn(OuterI) -> Result<InnerI, String>,
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
    F: Fn(OuterI) -> Result<InnerI, String>,
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
    F: Fn(SetOuter) -> Result<SetInner, String>,
{
    fn output(&self, state: &State) -> Result<Get, String> {
        self.conduit.output(state)
    }

    fn input(&self, state: &mut State, value: SetOuter) -> Result<(), String> {
        self.conduit.input(state, (self.f)(value)?)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
