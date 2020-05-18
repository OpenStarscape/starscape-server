use slotmap::DenseSlotMap;
use std::collections::HashSet;
use std::sync::RwLock;

use crate::body::Body;
use crate::entity::Entity;
use crate::plumbing::{Property, Store};
use crate::server::Connection;
use crate::ship::Ship;

new_key_type! {
    pub struct EntityKey;
    pub struct BodyKey;
    pub struct ShipKey;
    pub struct PropertyKey;
    pub struct ConnectionKey;
}

pub type PendingUpdates = RwLock<HashSet<PropertyKey>>;

/// The entire game state at a single point in time
pub struct State {
    /// Current time in seconds since the start of the game
    pub time: f64,
    /// An entity ties together the pieces of a complex object
    pub entities: DenseSlotMap<EntityKey, Entity>,
    /// All physics objects in the game
    pub bodies: Store<DenseSlotMap<BodyKey, Body>>,
    /// Keys to the bodies which have a gravitational force
    /// For performence reasons, only significantly massive bodies should be included
    pub gravity_wells: Vec<BodyKey>,
    /// Ships that can control themselves
    pub ships: DenseSlotMap<ShipKey, Ship>,
    /// Subscribers that need to be updated
    pub pending_updates: PendingUpdates,
    /// Object properties that may be subscribed to changes
    pub properties: DenseSlotMap<PropertyKey, Box<dyn Property>>,
    /// Network connections to clients
    pub connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
}

impl State {
    pub fn new() -> Self {
        State {
            time: 0.0,
            entities: DenseSlotMap::with_key(),
            bodies: Store::new(DenseSlotMap::with_key()),
            gravity_wells: Vec::new(),
            ships: DenseSlotMap::with_key(),
            pending_updates: RwLock::new(HashSet::new()),
            properties: DenseSlotMap::with_key(),
            connections: DenseSlotMap::with_key(),
        }
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
        assert!(self.entities.is_empty());
        assert!(self.bodies.is_empty());
        assert!(self.gravity_wells.is_empty());
        assert!(self.ships.is_empty());
        // pending_updates intentionally not checked
        assert!(self.properties.is_empty());
        assert!(self.connections.is_empty());
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
