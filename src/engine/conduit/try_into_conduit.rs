use super::*;

pub struct TryIntoConduit<C, GetInner, SetInner>(C, PhantomData<GetInner>, PhantomData<SetInner>);

impl<C, GetInner, SetInner> TryIntoConduit<C, GetInner, SetInner> {
    pub fn new(inner: C) -> Self {
        TryIntoConduit(inner, PhantomData, PhantomData)
    }
}

impl<C, GetInner, SetInner, GetOuter, SetOuter> Conduit<GetOuter, SetOuter>
    for TryIntoConduit<C, GetInner, SetInner>
where
    C: Conduit<GetInner, SetInner> + 'static,
    GetInner: Into<GetOuter> + 'static,
    SetInner: 'static,
    SetOuter: Into<Result<SetInner, String>>,
{
    fn get_value(&self, state: &State) -> Result<GetOuter, String> {
        self.0.get_value(state).map(Into::into)
    }

    fn set_value(&self, state: &mut State, value: SetOuter) -> Result<(), String> {
        match value.into() {
            Ok(value) => self.0.set_value(state, value),
            Err(e) => Err(format!(
                "failed to convert {} -> {}: {}",
                type_name::<SetOuter>(),
                type_name::<SetInner>(),
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
