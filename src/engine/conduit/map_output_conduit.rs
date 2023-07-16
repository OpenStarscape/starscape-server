use super::*;

pub struct MapOutputConduit<C, GetInner, Set, F> {
    conduit: C,
    f: F,
    get_pd: PhantomData<GetInner>,
    set_pd: PhantomData<Set>,
}

impl<C, F, GetInner, Set> MapOutputConduit<C, GetInner, Set, F> {
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
    F: Fn(&State, InnerO) -> RequestResult<OuterO> + Send + Sync,
    InnerO: Send + Sync,
    I: Send + Sync,
{
    fn output(&self, state: &State) -> RequestResult<OuterO> {
        (self.f)(state, self.conduit.output(state)?)
    }

    fn input(&self, state: &mut State, value: I) -> RequestResult<()> {
        self.conduit.input(state, value)
    }
}
impl<C, F, InnerO, I> Subscribable for MapOutputConduit<C, InnerO, I, F>
where
    C: Conduit<InnerO, I>,
{
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
