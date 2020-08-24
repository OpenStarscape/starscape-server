use super::*;

new_key_type! {
    pub struct BodyKey;
    pub struct ShipKey;
}

pub type PendingNotifications = RwLock<Vec<Weak<dyn Subscriber>>>;

/// The entire game state at a single point in time
pub struct State {
    /// Current time in seconds since the start of the game
    pub time: f64,
    pub entities: Box<dyn EntityStore>,
    /// All physics objects in the game
    pub bodies: UpdateSource<DenseSlotMap<BodyKey, Body>>,
    /// Keys to the bodies which have a gravitational force
    /// For performence reasons, only significantly massive bodies should be included
    pub gravity_wells: Vec<BodyKey>,
    /// Ships that can control themselves
    pub ships: DenseSlotMap<ShipKey, Ship>,
    /// Subscribers that need to be updated
    pub pending_updates: PendingNotifications,
}

impl Default for State {
    fn default() -> Self {
        Self {
            time: 0.0,
            entities: EntityStore::default_impl(),
            bodies: UpdateSource::new(DenseSlotMap::with_key()),
            gravity_wells: Vec::new(),
            ships: DenseSlotMap::with_key(),
            pending_updates: RwLock::new(Vec::new()),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a body to the game state
    /// A gravity well is automatically added if body.gravity_well is true
    pub fn add_body(&mut self, body: Body) -> BodyKey {
        let gravity = *body.gravity_well;
        let key = self.bodies.get_mut(&self.pending_updates).insert(body);
        if gravity {
            self.gravity_wells.push(key);
        }
        key
    }

    /// Remove a body from the game state and do any needed cleanup
    /// TODO: test
    pub fn remove_body(&mut self, body_key: BodyKey) -> Result<(), ()> {
        if let Some(body) = self.bodies.get_mut(&self.pending_updates).remove(body_key) {
            if *body.gravity_well {
                match self.gravity_wells.iter().position(|key| *key == body_key) {
                    None => eprintln!(
                        "Body {:?} thinks it has a gravity well, but it does not",
                        body_key
                    ),
                    Some(i) => {
                        self.gravity_wells.swap_remove(i);
                    }
                }
            }
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(test)]
    pub fn assert_is_empty(&self) {
        assert!(self.bodies.is_empty());
        assert!(self.gravity_wells.is_empty());
        assert!(self.ships.is_empty());
        // pending_updates intentionally not checked
    }
}

impl RequestHandler for State {
    fn set(&mut self, entity: EntityKey, property: &str, value: &Decodable) -> Result<(), String> {
        let property = self.entities.get_property(entity, property)?.clone();
        property.set_value(self, value)
    }

    fn get(&self, entity: EntityKey, property: &str) -> Result<Encodable, String> {
        let property = self.entities.get_property(entity, property)?;
        property.get_value(self)
    }

    fn subscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String> {
        let property = self.entities.get_property(entity, property)?;
        property.subscribe(self, connection)?;
        Ok(())
    }

    fn unsubscribe(
        &mut self,
        entity: EntityKey,
        property: &str,
        connection: ConnectionKey,
    ) -> Result<(), String> {
        let property = self.entities.get_property(entity, property)?;
        property.unsubscribe(self, connection)?;
        Ok(())
    }
}

/// Should only be used once per type per test
#[cfg(test)]
pub fn mock_keys<T: slotmap::Key>(number: u32) -> Vec<T> {
    let mut map = slotmap::DenseSlotMap::with_key();
    (0..number).map(|_| map.insert(())).collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use cgmath::Point3;

    #[test]
    fn add_body_adds_body() {
        let mut state = State::new();
        assert_eq!((*state.bodies).len(), 0);
        state.add_body(Body::new());
        assert_eq!(state.bodies.len(), 1);
        let key = state.add_body(Body::new().with_position(Point3::new(7.0, 0.0, 0.0)));
        state.add_body(Body::new());
        assert_eq!(state.bodies.len(), 3);
        assert_eq!(state.bodies[key].position.x, 7.0);
    }

    #[test]
    fn add_body_does_not_add_gravity_well_normally() {
        let mut state = State::new();
        assert_eq!(state.gravity_wells.len(), 0);
        state.add_body(Body::new());
        state.add_body(Body::new().with_position(Point3::new(7.0, 0.0, 0.0)));
        state.add_body(Body::new());
        assert_eq!(state.gravity_wells.len(), 0);
    }

    #[test]
    fn add_body_can_add_gravity_well() {
        let mut state = State::new();
        assert_eq!(state.gravity_wells.len(), 0);
        state.add_body(Body::new());
        state.add_body(
            Body::new()
                .with_position(Point3::new(7.0, 0.0, 0.0))
                .with_gravity(),
        );
        state.add_body(Body::new());
        assert_eq!(state.gravity_wells.len(), 1);
        assert_eq!(state.bodies[state.gravity_wells[0]].position.x, 7.0);
    }

    #[test]
    fn is_empty_by_default() {
        let state = State::new();
        state.assert_is_empty();
    }

    #[test]
    fn mock_keys_all_different() {
        let k: Vec<EntityKey> = mock_keys(3);
        assert_eq!(k.len(), 3);
        assert_ne!(k[0], k[1]);
        assert_ne!(k[0], k[2]);
        assert_ne!(k[1], k[2]);
    }
}
