use super::*;

pub struct MapOutputConduit<C, GetInner, Set, F> {
    conduit: C,
    f: F,
    get_pd: PhantomData<GetInner>,
    set_pd: PhantomData<Set>,
}

impl<C, F, GetInner, GetOuter, Set> MapOutputConduit<C, GetInner, Set, F>
where
    C: Conduit<GetInner, Set>,
    F: Fn(GetInner) -> Result<GetOuter, String>,
{
    pub fn new(conduit: C, f: F) -> Self {
        Self {
            conduit,
            f,
            get_pd: PhantomData,
            set_pd: PhantomData,
        }
    }
}

impl<C, F, InnerO, OuterO, I> Conduit<OuterO, I> for MapOutputConduit<C, InnerO, I, F>
where
    C: Conduit<InnerO, I>,
    F: Fn(InnerO) -> Result<OuterO, String>,
{
    fn output(&self, state: &State) -> Result<OuterO, String> {
        (self.f)(self.conduit.output(state)?)
    }

    fn input(&self, state: &mut State, value: I) -> Result<(), String> {
        self.conduit.input(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
