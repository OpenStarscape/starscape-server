use super::*;

pub struct TryIntoConduit<C, GetInner, SetInner>(C, PhantomData<GetInner>, PhantomData<SetInner>);

impl<C, GetInner, SetInner> TryIntoConduit<C, GetInner, SetInner> {
    pub fn new(inner: C) -> Self {
        TryIntoConduit(inner, PhantomData, PhantomData)
    }
}

impl<C, InnerO, InnerI, OuterO, OuterI> Conduit<OuterO, OuterI>
    for TryIntoConduit<C, InnerO, InnerI>
where
    C: Conduit<InnerO, InnerI> + 'static,
    InnerO: Into<OuterO> + 'static,
    InnerI: 'static,
    OuterI: Into<Result<InnerI, String>>,
{
    fn output(&self, state: &State) -> Result<OuterO, String> {
        self.0.output(state).map(Into::into)
    }

    fn input(&self, state: &mut State, value: OuterI) -> Result<(), String> {
        match value.into() {
            Ok(value) => self.0.input(state, value),
            Err(e) => Err(format!(
                "failed to convert {} -> {}: {}",
                type_name::<OuterI>(),
                type_name::<InnerI>(),
                e
            )),
        }
    }

    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> Result<(), String> {
        self.0.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> Result<(), String> {
        self.0.unsubscribe(state, subscriber)
    }
}

impl<C, GetInner, SetInner> From<C> for TryIntoConduit<C, GetInner, SetInner>
where
    C: Conduit<GetInner, SetInner>,
{
    fn from(src: C) -> Self {
        TryIntoConduit(src, PhantomData, PhantomData)
    }
}
