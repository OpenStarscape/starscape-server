use super::*;

new_key_type! {
    pub struct TypedKey;
    pub struct GenericKey;
}

#[derive(derivative::Derivative)]
#[derivative(Copy, Clone, PartialEq)]
pub struct Id<T> {
    typed_key: TypedKey,
    generic_key: GenericKey,
    /// This incantation is based on https://stackoverflow.com/a/50201389. It allows this struct to
    /// be associated with the type T while being Sync without owning a T or requiring T be Sync.
    /// Get rid of it if you can lol.
    phantom: PhantomData<dyn Fn() -> T + Send + Sync>,
}

impl<T> Id<T> {
    pub fn new(typed_key: TypedKey, generic_key: GenericKey) -> Self {
        Self {
            typed_key,
            generic_key,
            phantom: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self {
            typed_key: TypedKey::null(),
            generic_key: GenericKey::null(),
            phantom: PhantomData,
        }
    }

    pub fn is_null(&self) -> bool {
        self.typed_key.is_null()
    }
}

#[derive(derivative::Derivative)]
#[derivative(Copy, Clone, Hash, Eq, PartialEq)]
pub struct GenericId {
    typed_key: TypedKey,
    generic_key: GenericKey,
    type_id: TypeId,
    type_name: &'static str,
}

impl GenericId {
    pub fn new(generic_key: GenericKey, type_name: &'static str) -> Self {
        struct UnusedType;
        Self {
            typed_key: TypedKey::null(),
            generic_key,
            type_id: TypeId::of::<UnusedType>(),
            type_name,
        }
    }

    pub fn null() -> Self {
        Self::new(GenericKey::null(), "null")
    }

    pub fn key(&self) -> GenericKey {
        self.generic_key
    }

    pub fn is_null(&self) -> bool {
        self.generic_key.is_null()
    }
}

impl<T> AsRef<TypedKey> for Id<T> {
    fn as_ref(&self) -> &TypedKey {
        &self.typed_key
    }
}

impl<T> AsRef<GenericKey> for Id<T> {
    fn as_ref(&self) -> &GenericKey {
        &self.generic_key
    }
}

impl AsRef<GenericKey> for GenericId {
    fn as_ref(&self) -> &GenericKey {
        &self.generic_key
    }
}

impl<T> From<Id<T>> for GenericId
where
    T: 'static,
{
    fn from(typed: Id<T>) -> Self {
        Self {
            typed_key: typed.typed_key,
            generic_key: typed.generic_key,
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
        }
    }
}

impl<T> From<GenericId> for RequestResult<Id<T>>
where
    T: 'static,
{
    fn from(generic: GenericId) -> Self {
        if generic.type_id == TypeId::of::<T>() {
            Ok(Id {
                typed_key: generic.typed_key,
                generic_key: generic.generic_key,
                phantom: PhantomData,
            })
        } else {
            Err(RequestError::BadRequest(format!(
                "{:?} is of type {} but {} expected",
                generic.generic_key,
                generic.type_name,
                type_name::<T>()
            )))
        }
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", type_name::<T>(), self.typed_key.data().as_ffi())
    }
}

impl std::fmt::Debug for GenericId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.type_name, self.typed_key.data().as_ffi())
    }
}
