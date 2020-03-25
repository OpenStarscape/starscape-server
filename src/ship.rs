use cgmath::Point3;
use std::sync::{Arc, RwLock};

use super::body::{Body, Collision, Shape, Brain, Type};
use super::state::{BodyKey, State};

struct Ship {
    body: Option<BodyKey>,
}

impl Ship {
    fn new(state: &mut State, position: Point3<f64>) -> Arc<RwLock<Ship>> {
        let arc = Arc::new(RwLock::new(Ship { body: None }));
        let body = Body::new(Type::Ship, position, Shape::Sphere { radius: 1.0 }, Some(arc.clone()));
        let body_key = state.bodies.insert(body);
        {
            let mut ship = arc.write().unwrap();
            ship.body = Some(body_key);
        }
        arc
    }
}

pub fn new_ship(state: &mut State, position: Point3<f64>) {
    Ship::new(state, position);
}

impl Brain for RwLock<Ship> {
    fn collided_with(&self, _state: &State, _collision: &Collision) {}
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
        let key = arc.read().unwrap().body.unwrap();
        assert_eq!(state.bodies[key].position, pos);
    }

    #[test]
    fn sets_up_bodies_brain() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let arc = Ship::new(&mut state, pos);
        let key = arc.read().unwrap().body.unwrap();
        assert!(state.bodies[key].brain.is_some());
    }
}
