use cgmath::{Point3, Vector3};
use std::sync::{Arc, RwLock};

use super::body::{Body, Collision, Shape};
use super::object::Object;
use super::state::{BodyKey, ObjectKey, State};

struct Ship {
    position: Point3<f64>,
    velocity: Vector3<f64>,
    collision_shape: Shape,
    body_key: Option<BodyKey>,
    object_key: Option<ObjectKey>,
}

impl Ship {
    fn new(state: &mut State, position: Point3<f64>) -> Arc<RwLock<Ship>> {
        let arc = Arc::new(RwLock::new(Ship {
            position: position,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            collision_shape: Shape::Sphere { radius: 1.0 },
            body_key: None,
            object_key: None,
        }));
        let body_key = state.bodies.insert(Box::new(arc.clone()));
        let object_key = state.objects.insert(Box::new(arc.clone()));
        {
            let mut ship = arc.write().unwrap();
            ship.body_key = Some(body_key);
            ship.object_key = Some(object_key);
        }
        arc
    }
}

pub fn new_ship(state: &mut State, position: Point3<f64>) {
    Ship::new(state, position);
}

impl Body for Arc<RwLock<Ship>> {
    fn position(&self) -> Point3<f64> {
        let ship = self.read().unwrap();
        ship.position
    }

    fn velocity(&self) -> Vector3<f64> {
        let ship = self.read().unwrap();
        ship.velocity
    }

    fn collision_shape(&self) -> Shape {
        let ship = self.read().unwrap();
        ship.collision_shape
    }

    fn step(&self, _state: &State, _start_time: f64, delta_time: f64, _collisions: &[Collision]) {
        let mut ship = self.write().unwrap();
        ship.position = ship.position + (ship.velocity * delta_time);
        // TODO: apply gravity
    }
}

impl Object for Arc<RwLock<Ship>> {
    fn process_messages(&self, _messages: &str) {
        println!("process_messages() not yet implemented");
    }

    fn get_updates(&self) -> String {
        "get_updates() not yet implemented".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_body_to_state() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        assert_eq!(state.bodies.len(), 1);
        let key = arc.read().unwrap().body_key.unwrap();
        assert_eq!(state.bodies[key].position(), pos);
    }

    #[test]
    fn adds_object_to_state() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        assert_eq!(state.bodies.len(), 1);
        let key = arc.read().unwrap().body_key.unwrap();
        let _ = state.bodies[key];
    }

    #[test]
    fn step_applies_velocity() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let vel = Vector3::new(0.0, 1.0, 4.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        arc.write().unwrap().velocity = vel;
        arc.step(&state, 0.0, 1.0, &vec![]);
        assert_eq!(arc.position(), pos + vel);
    }

    #[test]
    fn step_applies_velocity_based_on_delta() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let vel = Vector3::new(0.0, 1.0, 4.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        arc.write().unwrap().velocity = vel;
        arc.step(&state, 4.0, 2.0, &vec![]);
        assert_eq!(arc.position(), pos + (vel * 2.0));
    }
}
