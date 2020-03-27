use slotmap::DenseSlotMap;

use super::body::Body;

new_key_type! {
    pub struct BodyKey;
}

/// The entire game state at a single point in time
pub struct State {
    /// Current time in seconds since the start of the game
    pub time: f64,
    /// A glorified Vec of all physics objects in the game
    pub bodies: DenseSlotMap<BodyKey, Body>,
    /// Keys to the bodies which have a gravitational force
    /// For performence reasons, only significantly massive bodies should be included
    pub gravity_wells: Vec<BodyKey>,
}

impl State {
    pub fn new() -> State {
        State {
            time: 0.0,
            bodies: DenseSlotMap::with_key(),
            gravity_wells: Vec::new(),
        }
    }

    /// Add a body to the game state
    /// A gravity well is automatically added if body.gravity_well is true
    pub fn add_body(&mut self, body: Body) -> BodyKey {
        let gravity = body.gravity_well;
        let key = self.bodies.insert(body);
        if gravity {
            self.gravity_wells.push(key);
        }
        key
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
}
