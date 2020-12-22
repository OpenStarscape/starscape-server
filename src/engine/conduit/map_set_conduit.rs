use super::*;

pub struct MapSetConduit<C, Get, SetInner, SetOuter, F>
where
    C: Conduit<Get, SetInner>,
    F: Fn(SetOuter) -> Result<SetInner, String>,
{
    conduit: C,
    f: F,
    get_pd: PhantomData<Get>,
    set_inner_pd: PhantomData<SetInner>,
    set_outer_pd: PhantomData<SetOuter>,
}

impl<C, F, Get, SetInner, SetOuter> MapSetConduit<C, Get, SetInner, SetOuter, F>
where
    C: Conduit<Get, SetInner>,
    F: Fn(SetOuter) -> Result<SetInner, String>,
{
    pub fn new(conduit: C, f: F) -> Self {
        MapSetConduit {
            conduit,
            f,
            get_pd: PhantomData,
            set_inner_pd: PhantomData,
            set_outer_pd: PhantomData,
        }
    }
}

impl<C, F, Get, SetInner, SetOuter> Conduit<Get, SetOuter>
    for MapSetConduit<C, Get, SetInner, SetOuter, F>
where
    C: Conduit<Get, SetInner>,
    F: Fn(SetOuter) -> Result<SetInner, String>,
{
    fn get_value(&self, state: &State) -> Result<Get, String> {
        self.conduit.get_value(state)
    }

    fn set_value(&self, state: &mut State, value: SetOuter) -> Result<(), String> {
        self.conduit.set_value(state, (self.f)(value)?)
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.conduit.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.conduit.unsubscribe(state, subscriber)
    }
}
