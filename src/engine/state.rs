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
    roots: Collection<game::Root>,
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

impl AsRef<Collection<game::Root>> for Data {
    fn as_ref(&self) -> &Collection<game::Root> {
        &self.roots
    }
}

impl AsMut<Collection<game::Root>> for Data {
    fn as_mut(&mut self) -> &mut Collection<game::Root> {
        &mut self.roots
    }
}

new_key_type! {
    /// A handle to an entity in the state. An entity is a collection of attached components. This
    /// key can be used to access those components from the State.
    pub struct EntityKey;
}

type ComponentMap<T> = HopSlotMap<ComponentKey<T>, (EntityKey, T)>;
type ComponentElement<T> = (PhantomData<T>, Element<()>);

/// Every game has one state. It owns all entities and components. Most code that uses the state
/// will be passed a reference to it. Entities and components inherit the state's mutability (if a
/// function is passed an immutable state, it can't change anything).
pub struct State {
    /// Current time in seconds since the start of the game
    time: f64,
    /// Monotonic clock that goes up with each physics tick
    physics_tick: u64,
    root: EntityKey,
    entities: HopSlotMap<EntityKey, Entity>,
    components: AnyMap,
    component_list_elements: Mutex<AnyMap>, // TODO: change to subscription trackers
    pub notif_queue: NotifQueue,
    data: Data,
    objects: SlotMap<id::GenericKey, Object>,
}

impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            time: 0.0,
            physics_tick: 0,
            root: EntityKey::null(),
            entities: HopSlotMap::with_key(),
            components: AnyMap::new(),
            component_list_elements: Mutex::new(AnyMap::new()),
            notif_queue: NotifQueue::new(),
            data: Data::default(),
            objects: SlotMap::default(),
        };
        state.root = state.create_entity();
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

    pub fn add_with_object<T>(&mut self, thing: T) -> (Id<T>, &mut Object)
    where
        Self: HasCollection<T>,
    {
        self.collection_mut().element.get_mut();
        let typed_key = self.collection_mut().map.insert(Thing::new(thing));
        let generic_key = self.objects.insert(Object::new(type_name::<T>()));
        self.collection_mut()
            .map
            .get_mut(typed_key)
            .unwrap()
            .generic = generic_key;
        let id = Id::new(typed_key, generic_key);
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
            None => Err(RequestError::BadId(id.into())),
        }
    }

    pub fn get<T: 'static>(&self, id: Id<T>) -> RequestResult<&T>
    where
        Self: HasCollection<T>,
    {
        match self.collection().map.get(*id.as_ref()) {
            Some(thing) => Ok(&thing.inner),
            None => Err(RequestError::BadId(id.into())),
        }
    }

    pub fn get_mut<T: 'static>(&mut self, id: Id<T>) -> RequestResult<&mut T>
    where
        Self: HasCollection<T>,
    {
        match self.collection_mut().map.get_mut(*id.as_ref()) {
            Some(thing) => Ok(&mut thing.inner),
            None => Err(RequestError::BadId(id.into())),
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
            None => Err(RequestError::BadId(id.into())),
        }
    }

    pub fn object<T>(&self, id: T) -> RequestResult<&Object>
    where
        T: AsRef<id::GenericKey> + Into<GenericId>,
    {
        self.objects
            .get(*id.as_ref())
            .ok_or(RequestError::BadId(id.into()))
    }

    pub fn object_mut<T>(&mut self, id: T) -> RequestResult<&mut Object>
    where
        T: AsRef<id::GenericKey> + Into<GenericId>,
    {
        self.objects
            .get_mut(*id.as_ref())
            .ok_or(RequestError::BadId(id.into()))
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

    /// Returns the key for the newly created entity
    pub fn create_entity(&mut self) -> EntityKey {
        self.entities.insert_with_key(Entity::new)
    }

    /// Returns the root entity, which is automatically created on construction. This will be the
    /// initial entity clients bind to.
    pub fn root_entity(&self) -> EntityKey {
        self.root
    }

    /// Current time in seconds since the start of the game
    pub fn time(&self) -> f64 {
        self.time
    }

    /*
    Hmm, this is a footgun because one might expect properties to always be the same on a given physics tick, but to
    make that so we'll need some sort of pending/committed concept.
    /// Monotonic clock that goes up with each physics tick
    pub fn physics_tick(&self) -> u64 {
        self.physics_tick
    }
    */

    /// Advance the physics tick by 1 and time by time_delta
    pub fn increment_physics(&mut self, time_delta: f64) {
        self.physics_tick += 1;
        self.time += time_delta;
        trace!(
            "Time advanced to {} (physics tick {})",
            self.time,
            self.physics_tick
        );
    }

    /// Removes the given entity and all its components from the state
    pub fn destroy_entity(&mut self, entity: EntityKey) -> Result<(), Box<dyn Error>> {
        let mut entity = self
            .entities
            .remove(entity)
            .ok_or_else(|| format!("destroy_entity() called on invalid entity {:?}", entity))?;
        entity.finalize(self);
        Ok(())
    }

    /// Attaches the new component to the given entity
    /// Panics if the entity already has a component of the given type
    pub fn install_component<T: 'static>(&mut self, entity: EntityKey, component: T) {
        let e = self
            .entities
            .get_mut(entity)
            .expect("can not add component to invalid entity");
        let map: &mut ComponentMap<T> =
            self.components.entry().or_insert_with(HopSlotMap::with_key);
        let key = map.insert((entity, component));
        e.register_component(key, move |state| state.remove_component(key));
        self.trigger_component_list_element_update::<T>();
        // TODO: test that an update is sent to the component list element
    }

    /// Returns the component of type T attached to the given entity
    /// or None if no such component is found
    pub fn component<T: 'static>(&self, entity: EntityKey) -> RequestResult<&T> {
        let e = self.entities.get(entity).ok_or(BadEntity(entity))?;
        let component = *e.component_key().ok_or_else(|| {
            InternalError(format!(
                "failed to get invalid component {} on entity {:?}",
                type_name::<T>(),
                entity
            ))
        })?;
        let map: &ComponentMap<T> = self
            .components
            .get()
            .ok_or_else(|| InternalError(format!("no components of type {}", type_name::<T>())))?;
        match map.get(component) {
            Some(v) => Ok(&v.1),
            None => Err(InternalError(format!(
                "invalid component {} ID {:?}",
                type_name::<T>(),
                component
            ))),
        }
    }

    /// Returns a mutable reference to the given component
    /// or None if no such component is found
    pub fn component_mut<T: 'static>(&mut self, entity: EntityKey) -> RequestResult<&mut T> {
        let e = self.entities.get(entity).ok_or(BadEntity(entity))?;
        let component = *e.component_key().ok_or_else(|| {
            InternalError(format!(
                "failed to get invalid component {} on entity {:?}",
                type_name::<T>(),
                entity
            ))
        })?;
        let map: &mut ComponentMap<T> = self
            .components
            .get_mut()
            .ok_or_else(|| InternalError(format!("no components of type {}", type_name::<T>())))?;
        match map.get_mut(component) {
            Some(v) => Ok(&mut v.1),
            None => Err(InternalError(format!(
                "invalid component {} ID {:?}",
                type_name::<T>(),
                component
            ))),
        }
    }

    /// Returns an iterator over all components of a particular type
    pub fn components_iter<'a, T: 'static>(
        &'a self,
    ) -> Box<dyn std::iter::Iterator<Item = (EntityKey, &T)> + 'a> {
        if let Some(map) = self.components.get::<ComponentMap<T>>() {
            Box::new(map.values().map(|(entity, value)| (*entity, value)))
        } else {
            Box::new(std::iter::empty())
        }
    }

    /// Returns a mutable iterator over all components of a particular type
    pub fn components_iter_mut<'a, T: 'static>(
        &'a mut self,
    ) -> Box<dyn std::iter::Iterator<Item = (EntityKey, &mut T)> + 'a> {
        match self.components.get_mut::<ComponentMap<T>>() {
            Some(map) => Box::new(map.values_mut().map(|(entity, value)| (*entity, value))),
            None => Box::new(std::iter::empty()),
        }
    }

    /// Subscribe to be notified when a component of type T is created or destroyed
    pub fn subscribe_to_component_list<T: 'static>(
        &self,
        subscriber: &Arc<dyn Subscriber>,
    ) -> RequestResult<()> {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.subscribe(self, subscriber)
    }

    pub fn unsubscribe_from_component_list<T: 'static>(
        &self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> RequestResult<()> {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.unsubscribe(self, subscriber)
    }

    /// Create a property for an entity. Panics if entity doesn't exist or already has something
    /// with this name.
    /// TODO: perhaps this shouldn't panic
    pub fn install_property<C>(&mut self, entity_key: EntityKey, name: &'static str, conduit: C)
    where
        C: Conduit<Value, Value> + 'static,
    {
        if let Some(entity) = self.entities.get_mut(entity_key) {
            let conduit = CachingConduit::new(conduit);
            entity.register_conduit(name, move |connection| {
                Ok(PropertyConduit::new(
                    connection,
                    entity_key,
                    name,
                    conduit.clone(),
                ))
            });
        } else {
            panic!(
                "failed to register property on invalid entity {:?}",
                entity_key
            );
        }
    }

    /// Create a signal for an entity. Panics if entity doesn't exist or already has something with
    /// this name.
    pub fn install_signal<C>(&mut self, entity_key: EntityKey, name: &'static str, conduit: C)
    where
        C: Conduit<Vec<Value>, SignalsDontTakeInputSilly> + 'static,
    {
        if let Some(entity) = self.entities.get_mut(entity_key) {
            let conduit =
                Arc::new(conduit) as Arc<dyn Conduit<Vec<Value>, SignalsDontTakeInputSilly>>;
            entity.register_conduit(name, move |connection| {
                Ok(SignalConduit::new(
                    connection,
                    entity_key,
                    name,
                    conduit.clone(),
                ))
            });
        } else {
            panic!(
                "failed to register signal on invalid entity {:?}",
                entity_key
            );
        }
    }

    /// Create an action for an entity. Panics if entity doesn't exist or already has something
    /// with this name.
    /// TODO: perhaps this shouldn't panic
    pub fn install_action<C>(&mut self, entity_key: EntityKey, name: &'static str, conduit: C)
    where
        C: Conduit<ActionsDontProduceOutputSilly, Value> + 'static,
    {
        if let Some(entity) = self.entities.get_mut(entity_key) {
            let conduit =
                Arc::new(conduit.map_output(|_| unreachable!())) as Arc<dyn Conduit<Value, Value>>;
            entity.register_conduit(name, move |connection| {
                Ok(PropertyConduit::new(
                    connection,
                    entity_key,
                    name,
                    conduit.clone(),
                ))
            });
        } else {
            panic!(
                "failed to register property on invalid entity {:?}",
                entity_key
            );
        }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        // pending_updates intentionally not checked
        self.components.is_empty()
            && self.entities.len() == 1
            && self.entities.get(self.root).is_some()
    }

    /// Returns the conduit for the property, signal or action with the given name, or the entitiy's
    /// destruction signal if name is None
    fn conduit(
        &self,
        connection: ConnectionKey,
        entity_key: EntityKey,
        name: &str,
    ) -> RequestResult<Box<dyn Conduit<Value, Value>>> {
        let entity = self.entities.get(entity_key).ok_or(BadEntity(entity_key))?;
        let conduit = entity
            .conduit(connection, name)
            .ok_or_else(|| BadName(entity_key, name.into()))??;
        Ok(conduit)
    }

    fn remove_component<T: 'static>(&mut self, component: ComponentKey<T>) {
        let mut remove_map = false;
        let mut update_component_list_element = false;
        match self.components.get_mut::<ComponentMap<T>>() {
            Some(map) => {
                if map.remove(component).is_some() {
                    update_component_list_element = true;
                } else {
                    error!("failed to remove {} {:?}", type_name::<T>(), component);
                }
                remove_map = map.is_empty();
            }
            None => {
                error!("no components of type {} to remove", type_name::<T>());
            }
        }

        if remove_map {
            self.components.remove::<ComponentMap<T>>();
        }

        if update_component_list_element {
            self.trigger_component_list_element_update::<T>();
        }
    }

    fn trigger_component_list_element_update<T: 'static>(&mut self) {
        let mut map = self
            .component_list_elements
            .lock()
            .expect("failed to lock component elements");
        let element = &mut map
            .entry::<ComponentElement<T>>()
            .or_insert_with(|| (PhantomData, Element::new(())))
            .1;
        element.get_mut();
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
        entity: EntityKey,
        name: &str,
        value: Value,
    ) -> RequestResult<()> {
        let conduit = self.conduit(connection, entity, name)?;
        // TODO: check if this is actually an action (currently "fireing" a property sets it)
        conduit.input(self, value)
    }

    fn set_property(
        &mut self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
        value: Value,
    ) -> RequestResult<()> {
        let conduit = self.conduit(connection, entity, name)?;
        // TODO: check if this is actually a property (currently "setting" an action fires it)
        conduit.input(self, value)
    }

    fn get_property(
        &self,
        connection: ConnectionKey,
        entity: EntityKey,
        name: &str,
    ) -> RequestResult<Value> {
        let conduit = self.conduit(connection, entity, name)?;
        conduit.output(self)
    }

    fn subscribe(
        &self,
        connection: ConnectionKey,
        entity_key: EntityKey,
        name: Option<&str>,
    ) -> RequestResult<Box<dyn Subscription>> {
        let conduit = if let Some(name) = name {
            self.conduit(connection, entity_key, name)?
        } else {
            let entity = self.entities.get(entity_key).ok_or(BadEntity(entity_key))?;
            let conduit = entity.destroyed_signal(&self.notif_queue);
            DestructionConduit::new(connection, entity_key, conduit)
        };
        let subscription = SubscriptionImpl::new(self, conduit)?;
        Ok(Box::new(subscription))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct MockComponent(i32);

    #[derive(Debug, PartialEq)]
    struct OtherMockComponent(bool);

    #[test]
    fn can_increment_physics() {
        let mut state = State::new();
        //assert_eq!(state.physics_tick(), 0);
        assert_eq!(state.time(), 0.0);
        state.increment_physics(1.0);
        //assert_eq!(state.physics_tick(), 1);
        assert_eq!(state.time(), 1.0);
        state.increment_physics(2.5);
        //assert_eq!(state.physics_tick(), 2);
        assert_eq!(state.time(), 3.5);
    }

    #[test]
    fn is_empty_by_default() {
        let state = State::new();
        assert!(state.is_empty());
    }

    #[test]
    fn not_empty_after_entity_created() {
        let mut state = State::new();
        let _ = state.create_entity();
        assert!(!state.is_empty());
    }

    #[test]
    fn is_empty_after_entity_created_and_destroyed() {
        let mut state = State::new();
        let e = state.create_entity();
        state.destroy_entity(e).unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn is_empty_after_entity_and_component_created_and_destroyed() {
        let mut state = State::new();
        let e = state.create_entity();
        state.install_component(e, MockComponent(3));
        state.destroy_entity(e).unwrap();
        assert!(state.is_empty());
    }

    #[test]
    #[should_panic(expected = "invalid entity")]
    fn panics_when_component_added_to_destroyed_entity() {
        let mut state = State::new();
        let e = state.create_entity();
        state.destroy_entity(e).unwrap();
        state.install_component(e, MockComponent(3));
    }

    #[test]
    #[should_panic(expected = "multiple")]
    fn panics_when_2nd_component_of_same_type_added_to_entity() {
        let mut state = State::new();
        let e = state.create_entity();
        state.install_component(e, MockComponent(3));
        state.install_component(e, MockComponent(4));
    }

    #[test]
    fn components_of_different_types_can_be_added_to_entity() {
        let mut state = State::new();
        let e = state.create_entity();
        state.install_component(e, MockComponent(3));
        state.install_component(e, OtherMockComponent(true));
    }

    #[test]
    fn can_get_component() {
        let mut state = State::new();
        let e = state.create_entity();
        state.install_component(e, MockComponent(3));
        assert_eq!(state.component::<MockComponent>(e), Ok(&MockComponent(3)));
    }

    #[test]
    fn multiple_entities_can_be_created_and_destroyed() {
        let mut state = State::new();
        let e0 = state.create_entity();
        let e1 = state.create_entity();
        state.destroy_entity(e1).unwrap();
        state.destroy_entity(e0).unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn can_get_components_from_multiple_entities() {
        let mut state = State::new();
        let e0 = state.create_entity();
        let e1 = state.create_entity();
        state.install_component(e0, MockComponent(0));
        state.install_component(e1, MockComponent(1));
        assert_eq!(state.component::<MockComponent>(e0), Ok(&MockComponent(0)));
        assert_eq!(state.component::<MockComponent>(e1), Ok(&MockComponent(1)));
    }

    #[test]
    fn getting_component_on_invalid_entity_is_err() {
        let state = State::new();
        let e = mock_keys(1);
        assert!(state.component::<MockComponent>(e[0]).is_err());
    }

    #[test]
    fn getting_invalid_component_is_err() {
        let mut state = State::new();
        let e = state.create_entity();
        assert!(state.component::<MockComponent>(e).is_err());
    }

    #[test]
    fn can_mutate_component() {
        let mut state = State::new();
        let e = state.create_entity();
        state.install_component(e, MockComponent(3));
        let mut c = state
            .component_mut::<MockComponent>(e)
            .expect("could not get component");
        c.0 = 5;
        assert_eq!(state.component::<MockComponent>(e), Ok(&MockComponent(5)));
    }

    // TODO: test component iterators
    // TODO: test subscribing to component list and getting updates
    // TODO: test installing properties
}
