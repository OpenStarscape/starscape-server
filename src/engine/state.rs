use super::*;

struct Thing<T> {
    inner: T,
    generic: id::GenericKey,
    cleanup: Vec<Box<dyn Fn(&mut State, &mut T)>>,
}

impl<T> Thing<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            generic: id::GenericKey::null(),
            cleanup: Vec::new(),
        }
    }
}

pub struct Collection<T> {
    map: HopSlotMap<id::TypedKey, Thing<T>>,
    element: Element<()>,
}

impl<T> Default for Collection<T> {
    fn default() -> Self {
        Self {
            map: HopSlotMap::default(),
            element: Element::default(),
        }
    }
}

#[derive(Default)]
struct Data {
    bodies: Collection<game::Body>,
    ships: Collection<game::Ship>,
}

impl AsRef<Collection<game::Body>> for Data {
    fn as_ref(&self) -> &Collection<game::Body> {
        &self.bodies
    }
}

impl AsMut<Collection<game::Body>> for Data {
    fn as_mut(&mut self) -> &mut Collection<game::Body> {
        &mut self.bodies
    }
}

impl AsRef<Collection<game::Ship>> for Data {
    fn as_ref(&self) -> &Collection<game::Ship> {
        &self.ships
    }
}

impl AsMut<Collection<game::Ship>> for Data {
    fn as_mut(&mut self) -> &mut Collection<game::Ship> {
        &mut self.ships
    }
}

/// Every game has one state. It owns all entities and components. Most code that uses the state
/// will be passed a reference to it. Entities and components inherit the state's mutability (if a
/// function is passed an immutable state, it can't change anything).
pub struct State {
    pub metronome: Metronome,
    root_id: GenericId,
    pub notif_queue: NotifQueue,
    data: Data,
    pub root: game::Root,
    objects: SlotMap<id::GenericKey, Object>,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            metronome: Metronome::default(),
            root_id: GenericId::null(),
            notif_queue: NotifQueue::new(),
            data: Data::default(),
            root: game::Root::default(),
            objects: SlotMap::default(),
        };
        (state.root_id, _) = state.add_object("Root");
        game::Root::install(&mut state);
        state
    }
}

pub trait HasCollection<T> {
    fn collection(&self) -> &Collection<T>;
    fn collection_mut(&mut self) -> &mut Collection<T>;
}

impl<T> HasCollection<T> for State
where
    Data: AsRef<Collection<T>>,
    Data: AsMut<Collection<T>>,
{
    fn collection(&self) -> &Collection<T> {
        self.data.as_ref()
    }

    fn collection_mut(&mut self) -> &mut Collection<T> {
        self.data.as_mut()
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_without_object<T>(&mut self, thing: T) -> Id<T>
    where
        Self: HasCollection<T>,
    {
        self.collection_mut().element.get_mut();
        let typed_key = self.collection_mut().map.insert(Thing::new(thing));
        Id::new(typed_key, id::GenericKey::null())
    }

    pub fn add_with_object<T: 'static>(&mut self, thing: T) -> (Id<T>, &mut Object)
    where
        Self: HasCollection<T>,
    {
        self.collection_mut().element.get_mut();
        let typed_key = self.collection_mut().map.insert(Thing::new(thing));
        let notif_queue = &self.notif_queue;
        let generic_key = self
            .objects
            .insert_with_key(|key| Object::new(Id::<T>::new(typed_key, key).into(), notif_queue));
        self.collection_mut()
            .map
            .get_mut(typed_key)
            .unwrap()
            .generic = generic_key;
        let id = Id::new(typed_key, generic_key);
        let obj = self.objects.get_mut(generic_key).unwrap();
        (id, obj)
    }

    pub fn add_object(&mut self, type_name: &'static str) -> (GenericId, &mut Object) {
        let notif_queue = &self.notif_queue;
        let generic_key = self
            .objects
            .insert_with_key(|key| Object::new(GenericId::new(key, type_name), notif_queue));
        // Use Object as the key type so that "Object" will be the type string used when the typed
        // id is turned into a generic id.
        let id = GenericId::new(generic_key, type_name);
        let obj = self.objects.get_mut(generic_key).unwrap();
        (id, obj)
    }

    pub fn remove<T: 'static>(&mut self, id: Id<T>) -> RequestResult<T>
    where
        Self: HasCollection<T>,
    {
        match self.collection_mut().map.remove(*id.as_ref()) {
            Some(mut thing) => {
                self.collection_mut().element.get_mut();
                for f in thing.cleanup.drain(..) {
                    f(self, &mut thing.inner);
                }
                if let Some(mut obj) = self.objects.remove(GenericId::from(id).key()) {
                    obj.finalize(self);
                }
                Ok(thing.inner)
            }
            None => Err(BadId(id.into())),
        }
    }

    pub fn get<T: 'static>(&self, id: Id<T>) -> RequestResult<&T>
    where
        Self: HasCollection<T>,
    {
        match self.collection().map.get(*id.as_ref()) {
            Some(thing) => Ok(&thing.inner),
            None => Err(BadId(id.into())),
        }
    }

    pub fn get_mut<T: 'static>(&mut self, id: Id<T>) -> RequestResult<&mut T>
    where
        Self: HasCollection<T>,
    {
        match self.collection_mut().map.get_mut(*id.as_ref()) {
            Some(thing) => Ok(&mut thing.inner),
            None => Err(BadId(id.into())),
        }
    }

    pub fn on_destroy<T: 'static, F>(&mut self, id: Id<T>, f: F) -> RequestResult<()>
    where
        Self: HasCollection<T>,
        F: Fn(&mut State, &mut T) + 'static,
    {
        match self.collection_mut().map.get_mut(*id.as_ref()) {
            Some(thing) => {
                thing.cleanup.push(Box::new(f));
                Ok(())
            }
            None => Err(BadId(id.into())),
        }
    }

    pub fn object<T>(&self, id: T) -> RequestResult<&Object>
    where
        T: AsRef<id::GenericKey> + Into<GenericId>,
    {
        self.objects.get(*id.as_ref()).ok_or(BadId(id.into()))
    }

    pub fn object_mut<T>(&mut self, id: T) -> RequestResult<&mut Object>
    where
        T: AsRef<id::GenericKey> + Into<GenericId>,
    {
        self.objects.get_mut(*id.as_ref()).ok_or(BadId(id.into()))
    }

    /// Subscribe to be notified when a component of type T is created or destroyed
    pub fn subscribe_to_collection<T: 'static>(
        &self,
        subscriber: &Arc<dyn Subscriber>,
    ) -> RequestResult<()>
    where
        Self: HasCollection<T>,
    {
        self.collection().element.subscribe(self, subscriber)
    }

    pub fn unsubscribe_from_collection<T: 'static>(
        &self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> RequestResult<()>
    where
        Self: HasCollection<T>,
    {
        self.collection().element.unsubscribe(self, subscriber)
    }

    pub fn iter<T: 'static>(&self) -> impl std::iter::Iterator<Item = (Id<T>, &T)>
    where
        Self: HasCollection<T>,
    {
        self.collection()
            .map
            .iter()
            .map(|(key, value)| (Id::new(key, value.generic), &value.inner))
    }

    pub fn iter_mut<T: 'static>(&mut self) -> impl std::iter::Iterator<Item = (Id<T>, &mut T)>
    where
        Self: HasCollection<T>,
    {
        self.collection_mut()
            .map
            .iter_mut()
            .map(|(key, value)| (Id::new(key, value.generic), &mut value.inner))
    }

    /// Returns the root entity, which is automatically created on construction. This will be the
    /// initial entity clients bind to.
    pub fn root(&self) -> GenericId {
        self.root_id
    }
}

impl AsRef<dyn Any> for State {
    fn as_ref(&self) -> &dyn Any {
        self
    }
}

impl RequestHandler for State {
    fn fire_action(
        &mut self,
        connection: ConnectionKey,
        object: GenericId,
        name: &str,
        value: Value,
    ) -> RequestResult<()> {
        let conduit = self
            .object(object)?
            .conduit(connection, &[MemberType::Action], name)?;
        // TODO: check if this is actually an action (currently "fireing" a property sets it)
        conduit.input(self, value)
    }

    fn set_property(
        &mut self,
        connection: ConnectionKey,
        object: GenericId,
        name: &str,
        value: Value,
    ) -> RequestResult<()> {
        let conduit = self
            .object(object)?
            .conduit(connection, &[MemberType::Property], name)?;
        // TODO: check if this is actually a property (currently "setting" an action fires it)
        conduit.input(self, value)
    }

    fn get_property(
        &self,
        connection: ConnectionKey,
        object: GenericId,
        name: &str,
    ) -> RequestResult<Value> {
        let conduit = self
            .object(object)?
            .conduit(connection, &[MemberType::Property], name)?;
        conduit.output(self)
    }

    fn subscribe(
        &self,
        connection: ConnectionKey,
        object: GenericId,
        name: Option<&str>,
    ) -> RequestResult<Box<dyn Subscription>> {
        let conduit = if let Some(name) = name {
            self.object(object)?.conduit(
                connection,
                &[MemberType::Property, MemberType::Signal],
                name,
            )?
        } else {
            let signal = self.object(object)?.destroyed_signal(&self.notif_queue);
            DestructionConduit::new(connection, object, signal)
        };
        let subscription = SubscriptionImpl::new(self, conduit)?;
        Ok(Box::new(subscription))
    }
}
