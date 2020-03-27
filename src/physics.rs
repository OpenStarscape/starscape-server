use cgmath::{MetricSpace, Point3};

use crate::state::State;

const EPSILON: f64 = 0.0001; // 10cm

/// TODO: calculate the gravitational constant for our units (kilometers, kilotonnes, seconds)
const GRAVITATIONAL_CONSTANT: f64 = 1.0;

pub fn apply_gravity(state: &mut State, dt: f64) {
    // we can't access the body (and thus the position) of a gravity well while we are mutating the
    // position of bodies, so we collect all the info we need into a local vec (which should be
    // good for performence as well)
    struct GravityWell {
        position: Point3<f64>,
        mass: f64,
    };
    let wells: Vec<GravityWell> = state
        .gravity_wells
        .iter()
        .map(|body| GravityWell {
            position: state.bodies[*body].position,
            mass: state.bodies[*body].mass,
        })
        .collect();
    state.bodies.values_mut().for_each(|body| {
        wells.iter().for_each(|well| {
            let distance2 = well.position.distance2(body.position);
            if distance2 > EPSILON {
                let acceleration = GRAVITATIONAL_CONSTANT * well.mass / distance2;
                let distance = distance2.sqrt();
                let normalized_vec = (well.position - body.position) / distance;
                body.velocity += (acceleration * dt) * normalized_vec;
            }
        })
    });
}

pub fn apply_motion(state: &mut State, dt: f64) {
    state.bodies.values_mut().for_each(|body| {
        body.position += dt * body.velocity;
    });
}

#[cfg(test)]
mod gravity_tests {
    use super::*;
    use crate::body::Body;
    use cgmath::Vector3;

    const PLANET_MASS: f64 = 5.972e+18; // mass of earth

    #[test]
    fn lone_gravity_body_is_unaffected() {
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        assert_eq!(state.bodies[body].velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(state.bodies[body].velocity, velocity);
    }

    #[test]
    fn lone_gravity_body_off_origin_is_unaffected() {
        let position = Point3::new(4.0, 2.0, 6.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity().with_position(position));
        assert_eq!(state.bodies[body].velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(state.bodies[body].velocity, velocity);
    }

    #[test]
    fn body_falls_towards_gravity_source() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);
        let mut state = State::new();
        state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body = state.add_body(Body::new().with_position(position));
        assert_eq!(state.bodies[body].velocity.x, 0.0);
        apply_gravity(&mut state, 1.0);
        let v = state.bodies[body].velocity;
        assert!(v.x < -EPSILON);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
    }

    #[test]
    fn acceleration_proportional_to_dt() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);

        let mut state_a = State::new();
        state_a.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body_a = state_a.add_body(Body::new().with_position(position));
        apply_gravity(&mut state_a, 1.0);
        let v_a = state_a.bodies[body_a].velocity;

        let mut state_b = State::new();
        state_b.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body_b = state_b.add_body(Body::new().with_position(position));
        apply_gravity(&mut state_b, 0.5);
        let v_b = state_b.bodies[body_b].velocity;

        assert!((v_a.x - (v_b.x * 2.0)).abs() < EPSILON);
    }

    #[test]
    fn falls_in_correct_direction() {
        let position = Point3::new(20.0e+3, 0.0, -20.0e+3);
        let mut state = State::new();
        state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body = state.add_body(Body::new().with_position(position));
        apply_gravity(&mut state, 1.0);
        let v = state.bodies[body].velocity;
        assert!(v.x < -EPSILON);
        assert!(v.y.abs() < EPSILON);
        assert!(v.z > EPSILON);
        assert!((v.x + v.z).abs() < EPSILON);
    }

    #[test]
    fn multiple_wells_cancel_each_other_out() {
        let position = Point3::new(-20.0e+3, 27.5, 154.0);
        let mut state = State::new();
        state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        state.add_body(
            Body::new()
                .with_mass(PLANET_MASS)
                .with_gravity()
                .with_position(position * 2.0),
        );
        let body = state.add_body(Body::new().with_position(position));
        apply_gravity(&mut state, 1.0);
        let v = state.bodies[body].velocity;
        assert!(v.x.abs() < EPSILON);
        assert!(v.y.abs() < EPSILON);
        assert!(v.z.abs() < EPSILON);
    }
}


#[cfg(test)]
mod motion_tests {
    use super::*;
    use crate::body::Body;
    use cgmath::Vector3;

    #[test]
    fn no_motion_if_zero_velocity() {
        let mut state = State::new();
        let body = state.add_body(Body::new());
        assert_eq!(state.bodies[body].position, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(state.bodies[body].velocity, Vector3::new(0.0, 0.0, 0.0));
        apply_motion(&mut state, 1.0);
        assert_eq!(state.bodies[body].position, Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn moves_bodies_by_velocity_amount() {
        let mut state = State::new();
        let body1 = state.add_body(Body::new().with_position(Point3::new(-1.0, 4.0, 0.0)).with_velocity(Vector3::new(1.0, 0.0, 2.0)));
        let body2 = state.add_body(Body::new().with_velocity(Vector3::new(0.0, 0.5, 0.0)));
        apply_motion(&mut state, 1.0);
        assert_eq!(state.bodies[body1].position, Point3::new(0.0, 4.0, 2.0));
        assert_eq!(state.bodies[body2].position, Point3::new(0.0, 0.5, 0.0));
    }

    #[test]
    fn respects_dt() {
        let mut state = State::new();
        let body = state.add_body(Body::new().with_velocity(Vector3::new(4.0, 0.0, 0.0)));
        apply_motion(&mut state, 0.5);
        assert_eq!(state.bodies[body].position, Point3::new(2.0, 0.0, 0.0));
    }
}
