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
    InnerO: Into<OuterO> + Send + Sync + 'static,
    InnerI: Send + Sync + 'static,
    OuterI: Into<RequestResult<InnerI>>,
{
    fn output(&self, state: &State) -> RequestResult<OuterO> {
        self.0.output(state).map(Into::into)
    }

    fn input(&self, state: &mut State, value: OuterI) -> RequestResult<()> {
        match value.into() {
            Ok(value) => self.0.input(state, value),
            Err(e) => Err(BadRequest(format!(
                "failed to convert {} -> {}: {}",
                short_type_name::<OuterI>(),
                short_type_name::<InnerI>(),
                e
            ))),
        }
    }
}
impl<C, InnerO, InnerI> Subscribable for TryIntoConduit<C, InnerO, InnerI>
where
    C: Conduit<InnerO, InnerI> + 'static,
    InnerI: Send + Sync + 'static,
{
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.0.subscribe(state, subscriber)
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
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
