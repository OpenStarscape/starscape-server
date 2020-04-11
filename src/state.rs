use slotmap::DenseSlotMap;
use std::collections::HashSet;
use std::sync::RwLock;

use crate::body::Body;
use crate::conduit::Conduit;
use crate::connection::Connection;
use crate::entity::Entity;
use crate::ship::Ship;

new_key_type! {
    pub struct EntityKey;
    pub struct BodyKey;
    pub struct ShipKey;
    pub struct ConduitKey;
    pub struct ConnectionKey;
}

pub type PendingUpdates = RwLock<HashSet<ConduitKey>>;

/// The entire game state at a single point in time
pub struct State {
    /// Current time in seconds since the start of the game
    pub time: f64,
    /// An entity ties together the pieces of a complex object (such as a ship)
    pub entities: DenseSlotMap<EntityKey, Box<dyn Entity>>,
    /// All physics objects in the game
    pub bodies: DenseSlotMap<BodyKey, Body>,
    /// Keys to the bodies which have a gravitational force
    /// For performence reasons, only significantly massive bodies should be included
    pub gravity_wells: Vec<BodyKey>,
    /// Ships that can control themselves
    pub ships: DenseSlotMap<ShipKey, Ship>,
    /// Subscribers that need to be updated
    pub pending_updates: PendingUpdates,
    /// Object properties that may be subscribed to changes
    pub conduits: DenseSlotMap<ConduitKey, Box<dyn Conduit>>,
    /// Network connections to clients
    pub connections: DenseSlotMap<ConnectionKey, Box<dyn Connection>>,
}

impl State {
    pub fn new() -> Self {
        State {
            time: 0.0,
            entities: DenseSlotMap::with_key(),
            bodies: DenseSlotMap::with_key(),
            gravity_wells: Vec::new(),
            ships: DenseSlotMap::with_key(),
            pending_updates: RwLock::new(HashSet::new()),
            conduits: DenseSlotMap::with_key(),
            connections: DenseSlotMap::with_key(),
        }
    }

    /// Add a body to the game state
    /// A gravity well is automatically added if body.gravity_well is true
    pub fn add_body(&mut self, body: Body) -> BodyKey {
        let gravity = *body.gravity_well;
        let key = self.bodies.insert(body);
        if gravity {
            self.gravity_wells.push(key);
        }
        key
    }

    #[cfg(test)]
    pub fn assert_is_empty(&self) {
        assert!(self.entities.is_empty());
        assert!(self.bodies.is_empty());
        assert!(self.gravity_wells.is_empty());
        assert!(self.ships.is_empty());
        // pending_updates intentionally not checked
        assert!(self.conduits.is_empty());
        assert!(self.connections.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Point3;

    #[test]
    fn add_body_adds_body() {
        let mut state = State::new();
        assert_eq!(state.bodies.len(), 0);
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
        let mut state = State::new();
        state.assert_is_empty();
    }
}
