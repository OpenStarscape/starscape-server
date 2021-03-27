use super::*;

pub struct OrbitData {
    semi_major: Vector3<f64>,
    semi_minor: Vector3<f64>,
    parent: EntityKey,
}

impl From<OrbitData> for Value {
    fn from(orbit: OrbitData) -> Self {
        let array: Vec<Value> = vec![
            orbit.semi_major.into(),
            orbit.semi_minor.into(),
            orbit.parent.into(),
        ];
        array.into()
    }
}

/// A conduit that implements a body's orbit property
pub struct OrbitConduit {
    subscribers: SyncSubscriberList,
    body: EntityKey,
    cached_parent: Mutex<EntityKey>,
}

impl OrbitConduit {
    pub fn new(body: EntityKey) -> Self {
        Self {
            subscribers: SyncSubscriberList::new(),
            body,
            cached_parent: Mutex::new(EntityKey::null()),
        }
    }

    fn for_each_parent_subscribable<F: Fn(&dyn Subscribable)>(
        state: &State,
        parent: EntityKey,
        f: &F,
    ) -> RequestResult<()> {
        let parent_body = state.component::<Body>(parent)?;
        f(&parent_body.position);
        f(&parent_body.velocity);
        f(&parent_body.mass);
        Ok(())
    }

    fn for_each_subscribable<F: Fn(&dyn Subscribable)>(
        &self,
        state: &State,
        f: &F,
    ) -> RequestResult<()> {
        let body = state.component::<Body>(self.body)?;
        Self::for_each_parent_subscribable(state, *self.cached_parent.lock().unwrap(), f)?;
        f(&body.gravity_parent);
        f(&body.position);
        f(&body.velocity);
        f(&body.mass);
        Ok(())
    }

    fn update_parent(&self, state: &State) {
        let parent = *state
            .component::<Body>(self.body)
            .expect("OrbitConduit body does not exist")
            .gravity_parent;
        let mut cached_parent = self.cached_parent.lock().unwrap();
        if parent != *cached_parent {
            let _ = Self::for_each_parent_subscribable(state, *cached_parent, &mut |s| {
                self.subscribers.unsubscribe_all(state, s);
            });
            *cached_parent = parent;
            let _ = Self::for_each_parent_subscribable(state, *cached_parent, &mut |s| {
                self.subscribers.subscribe_all(state, s);
            });
        }
    }
}

impl Conduit<OrbitData, ReadOnlyPropSetType> for OrbitConduit {
    fn output(&self, state: &State) -> RequestResult<OrbitData> {
        self.update_parent(state);
        Ok(OrbitData {
            semi_major: Vector3::zero(),
            semi_minor: Vector3::zero(),
            parent: *self.cached_parent.lock().unwrap(),
        })
    }

    fn input(&self, _: &mut State, _: ReadOnlyPropSetType) -> RequestResult<()> {
        unreachable!()
    }
}

impl Subscribable for OrbitConduit {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        self.update_parent(state);
        self.for_each_subscribable(state, &|s| {
            s.subscribe(state, subscriber)
                .or_log_error("subscribing to OrbitConduit");
        })?;
        self.subscribers.add(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        self.for_each_subscribable(state, &|s| {
            s.unsubscribe(state, subscriber)
                .or_log_error("unsubscribing from OrbitConduit");
        })?;
        self.subscribers.remove(subscriber)?;
        Ok(())
    }
}

// TODO: test
