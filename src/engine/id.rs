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
            type_name: short_type_name::<T>(),
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
                short_type_name::<T>()
            )))
        }
    }
}

impl<T: 'static> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", GenericId::from(*self))
    }
}

impl Debug for GenericId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let raw_id = if !self.typed_key.is_null() {
            Some(self.typed_key.data().as_ffi())
        } else if !self.generic_key.is_null() {
            Some(self.generic_key.data().as_ffi())
        } else {
            None
        };
        if let Some(raw_id) = raw_id {
            // This depends on undefined SlotMap internals, but whatever. No big deal if it breaks.
            let version = raw_id >> 32;
            let index = (raw_id << 32) >> 32;
            write!(f, "{}#{}:{}", self.type_name, index, version)
        } else {
            write!(f, "{}#null", self.type_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_keys() -> (TypedKey, GenericKey) {
        let mut map_a = SlotMap::<TypedKey, ()>::with_key();
        let mut map_b = SlotMap::<GenericKey, ()>::with_key();
        for _ in 0..7 {
            map_a.insert(());
        }
        for _ in 0..12 {
            map_b.insert(());
        }
        (map_a.insert(()), map_b.insert(()))
    }

    #[test]
    fn can_convert_from_generic_with_correct_type() {
        let (a, b) = get_keys();
        struct Foo;
        assert!(RequestResult::<Id<Foo>>::from(GenericId::from(Id::<Foo>::new(a, b))).is_ok());
    }

    #[test]
    fn converting_from_generic_fails_if_type_wrong() {
        let (a, b) = get_keys();
        struct Foo;
        mod bar {
            pub struct Foo;
        }
        assert!(
            RequestResult::<Id<bar::Foo>>::from(GenericId::from(Id::<Foo>::new(a, b))).is_err()
        );
    }

    #[test]
    fn converting_to_generic_and_back_does_not_break_equality() {
        struct Foo;
        let (a, b) = get_keys();
        let id = Id::<Foo>::new(a, b);
        assert_eq!(
            id,
            RequestResult::<Id<Foo>>::from(GenericId::from(id)).unwrap()
        );
    }

    #[test]
    fn is_null_different_for_typed_and_generic_ids() {
        let (a, b) = get_keys();
        struct Foo;
        let typed_null = Id::<Foo>::new(TypedKey::null(), b);
        assert!(typed_null.is_null());
        assert!(!GenericId::from(typed_null).is_null());
        let generic_null = Id::<Foo>::new(a, GenericKey::null());
        assert!(!generic_null.is_null());
        assert!(GenericId::from(generic_null).is_null());
    }

    #[test]
    fn debugs_to_short_name_with_typed_index_and_version() {
        let mut map_a = SlotMap::<TypedKey, ()>::with_key();
        for _ in 0..100 {
            map_a.insert(());
        }
        for _ in 0..10 {
            let key = map_a.insert(());
            map_a.remove(key);
        }
        let key = map_a.insert(());
        // How slotmap comes up with this I have no idea, but it should match what we get
        assert_eq!(format!("{:?}", key), "TypedKey(101v21)");
        struct Foo;
        let (_, generic) = get_keys();
        let id = Id::<Foo>::new(key, generic);
        assert_eq!(format!("{:?}", id), "Foo#101:21");
        assert_eq!(format!("{:?}", GenericId::from(id)), "Foo#101:21");
    }

    #[test]
    fn debugs_to_short_name_with_generic_index_and_version_when_typed_key_is_null() {
        let mut map_a = SlotMap::<GenericKey, ()>::with_key();
        for _ in 0..100 {
            map_a.insert(());
        }
        for _ in 0..10 {
            let key = map_a.insert(());
            map_a.remove(key);
        }
        let key = map_a.insert(());
        // How slotmap comes up with this I have no idea, but it should match what we get
        assert_eq!(format!("{:?}", key), "GenericKey(101v21)");
        struct Foo;
        let id = Id::<Foo>::new(TypedKey::null(), key);
        assert_eq!(format!("{:?}", id), "Foo#101:21");
        assert_eq!(format!("{:?}", GenericId::from(id)), "Foo#101:21");
    }

    #[test]
    fn null_id_debugs_correctly() {
        struct Foo;
        let id = Id::<Foo>::null();
        assert_eq!(format!("{:?}", id), "Foo#null");
        assert_eq!(format!("{:?}", GenericId::from(id)), "Foo#null");
    }
}
