use cgmath::*;
use slotmap::Key;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::body::{Body, Collision, Controller};
use crate::entity::Entity;
use crate::plumbing::new_property;
use crate::state::{BodyKey, EntityKey, PropertyKey, ShipKey, State};
use crate::EPSILON;

struct PendingUpdates {
    thrust: Vector3<f64>,
    kill: bool,
}

impl PendingUpdates {
    fn new() -> Self {
        Self {
            thrust: Vector3::zero(),
            kill: false,
        }
    }
}

pub struct Ship {
    max_thrust: f64,
    thrust: Vector3<f64>,
    alive: bool,
    pending: Mutex<PendingUpdates>,
}

impl Ship {
    fn new(max_thrust: f64) -> Self {
        Self {
            max_thrust,
            thrust: Vector3::zero(),
            alive: true,
            pending: Mutex::new(PendingUpdates::new()),
        }
    }

    fn set_thrust(&self, thrust: Vector3<f64>) -> Result<(), String> {
        let magnitude = thrust.magnitude();
        if magnitude > self.max_thrust + EPSILON {
            Err(format!(
                "{:?} has a magnitude of {}, which is greater than the maximum allowed thrust {}",
                thrust, magnitude, self.max_thrust
            ))
        } else {
            let mut pending = self
                .pending
                .lock()
                .expect("Failed lock pending ship updates");
            pending.thrust = thrust;
            Ok(())
        }
    }

    fn kill(&self) {
        let mut pending = self
            .pending
            .lock()
            .expect("Failed lock pending ship updates");
        pending.kill = true;
    }
}

struct ShipBodyController {
    ship: ShipKey,
}

impl Controller for ShipBodyController {
    fn collided_with(&self, state: &State, _collision: &Collision) {
        if let Some(ship) = state.ships.get(self.ship) {
            ship.kill();
        } else {
            eprint!("Could not find colliding ship {:?}", self.ship);
        }
    }
}

struct ShipEntity {
    entity: EntityKey,
    body: BodyKey,
    ship: ShipKey,
    properties: HashMap<&'static str, PropertyKey>,
}

impl ShipEntity {
    fn new(entity: EntityKey, body: BodyKey, ship: ShipKey) -> Self {
        Self {
            entity,
            body,
            ship,
            properties: HashMap::new(),
        }
    }
}

impl Entity for ShipEntity {
    fn add_property(&mut self, name: &'static str, key: PropertyKey) {
        self.properties.insert(name, key);
    }

    fn property(&self, name: &str) -> Result<PropertyKey, String> {
        if let Some(conduit) = self.properties.get(name) {
            Ok(*conduit)
        } else {
            Err(format!("Ship does not have a {:?} property", name))
        }
    }

    fn destroy(&mut self, state: &mut State) {
        state.bodies.remove(self.body);
        state.ships.remove(self.ship);
    }
}

pub fn create_ship(state: &mut State, position: Point3<f64>) -> EntityKey {
    let ship = state.ships.insert(Ship::new(10.0));
    let body = state.add_body(
        Body::new()
            .with_position(position)
            .with_sphere_shape(1.0)
            .with_controller(Box::new(ShipBodyController { ship })),
    );
    let entity = state
        .entities
        .insert_with_key(|entity| Box::new(ShipEntity::new(entity, body, ship)));
    new_property(state, entity, "position", move |state: &State| {
        Ok(&state.bodies[body].position)
    });
    entity
    /*for (name, prop_getter) in vec![
        ("position", &|state: &State| Ok(&state.bodies.get(body)?.position)),
        ("thrust", &|state: &State| Ok(&state.ships.get(ship)?.thrust)),
    ] {
        let conduit = state.conduits.insert(Box::new(PropertyConduit::new(entity, name, prop_getter)));
        state.entities[entity].add_conduit(name, conduit);
    }*/
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_up_when_destroyed() {
        panic!("Test not finished");
        let mut state = State::new();
        state.assert_is_empty();
    }

    #[test]
    fn body_has_correct_position() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        assert_eq!(state.bodies.len(), 1);
        let key = arc.read().unwrap().body.unwrap();
        assert_eq!(state.bodies[key].position, pos);
    }

    #[test]
    fn body_has_sphere_shape() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        assert_eq!(state.bodies.len(), 1);
        let key = arc.read().unwrap().body.unwrap();
        assert!(state.bodies[key].shape == crate::body::Shape::Sphere { radius: 1.0 });
    }

    #[test]
    fn sets_up_controller() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        let key = arc.read().unwrap().body.unwrap();
        assert_eq!(
            &*state.bodies[key].controller as *const _ as *const usize,
            &*arc as *const _ as *const usize,
        );
    }
}
*/
