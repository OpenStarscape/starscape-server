use super::*;

#[derive(Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct ComponentKey<T> {
    data: slotmap::KeyData,
    phantom: PhantomData<*const T>,
}

// Required because of https://github.com/rust-lang/rust/issues/26925
impl<T> Clone for ComponentKey<T> {
    fn clone(&self) -> Self {
        ComponentKey::<T> {
            data: self.data,
            phantom: self.phantom,
        }
    }
}

// Required because of https://github.com/rust-lang/rust/issues/26925
impl<T> Copy for ComponentKey<T> {}

// Required because of https://github.com/rust-lang/rust/issues/26925
impl<T> std::fmt::Debug for ComponentKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data)
    }
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

impl<T: Sized> slotmap::Key for ComponentKey<T> {}
