use cgmath::Point3;
use std::sync::{Arc, RwLock};

use super::body::{Body, Collision, Controller};
use super::state::{BodyKey, State};

struct Ship {
    body: Option<BodyKey>,
}

impl Ship {
    fn new(state: &mut State, position: Point3<f64>) -> Arc<RwLock<Ship>> {
        let arc = Arc::new(RwLock::new(Ship { body: None }));
        let body = Body::new()
            .with_position(position)
            .with_sphere_shape(1.0)
            .with_controller(arc.clone());
        let body_key = state.add_body(body);
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

impl Controller for RwLock<Ship> {
    fn collided_with(&self, _state: &State, _collision: &Collision) {}
}

#[cfg(test)]
mod tests {
    use super::*;

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
