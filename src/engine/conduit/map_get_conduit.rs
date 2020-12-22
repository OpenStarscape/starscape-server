use super::*;

pub struct MapGetConduit<C, GetInner, Set, F> {
    conduit: C,
    f: F,
    get_pd: PhantomData<GetInner>,
    set_pd: PhantomData<Set>,
}

impl<C, F, GetInner, GetOuter, Set> MapGetConduit<C, GetInner, Set, F>
where
    C: Conduit<GetInner, Set>,
    F: Fn(GetInner) -> Result<GetOuter, String>,
{
    pub fn new(conduit: C, f: F) -> Self {
        MapGetConduit {
            conduit,
            f,
            get_pd: PhantomData,
            set_pd: PhantomData,
        }
    }
}

impl<C, F, GetInner, GetOuter, Set> Conduit<GetOuter, Set> for MapGetConduit<C, GetInner, Set, F>
where
    C: Conduit<GetInner, Set>,
    F: Fn(GetInner) -> Result<GetOuter, String>,
{
    fn get_value(&self, state: &State) -> Result<GetOuter, String> {
        (self.f)(self.conduit.get_value(state)?)
    }

    fn set_value(&self, state: &mut State, value: Set) -> Result<(), String> {
        self.conduit.set_value(state, value)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
