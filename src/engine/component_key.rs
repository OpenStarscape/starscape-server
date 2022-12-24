use super::*;

/// A handle to a component, only used within the engine module. Outside the engine components are
/// referred to by their type and EntityKey. We ise derivitive crate instead of normal derive
/// because of https://github.com/rust-lang/rust/issues/26925.
#[repr(transparent)]
#[derive(derivative::Derivative)]
#[derivative(Default, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Debug, Clone)]
pub struct ComponentKey<T> {
    data: slotmap::KeyData,
    phantom: PhantomData<*const T>,
}

impl<T> From<slotmap::KeyData> for ComponentKey<T> {
    fn from(k: slotmap::KeyData) -> Self {
        Self {
            data: k,
            phantom: PhantomData,
        }
    }
}

impl<T> From<ComponentKey<T>> for slotmap::KeyData {
    fn from(k: ComponentKey<T>) -> Self {
        k.data
    }
}

/// All that is required for safety is that "all methods must behave exactly as if we’re operating
/// on a KeyData directly"
unsafe impl<T: Sized> slotmap::Key for ComponentKey<T> {
    fn data(&self) -> slotmap::KeyData {
        self.data
    }
}
