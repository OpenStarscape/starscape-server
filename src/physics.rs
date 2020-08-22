use cgmath::*;

use crate::components::{Body, Collision};
use crate::state::State;
use crate::EPSILON;

/// TODO: calculate the gravitational constant for our units (kilometers, kilotonnes, seconds)
const GRAVITATIONAL_CONSTANT: f64 = 6.674_301_5e-3;

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
            // TODO: error handing on body not in bodies
            position: *state.bodies[*body].position,
            mass: *state.bodies[*body].mass,
        })
        .collect();
    let pending_updates = &state.pending_updates;
    state
        .bodies
        .get_mut_without_notifying_of_change()
        .values_mut()
        .for_each(|body| {
            wells.iter().for_each(|well| {
                let distance2 = well.position.distance2(*body.position);
                if distance2 > EPSILON {
                    let acceleration = GRAVITATIONAL_CONSTANT * well.mass / distance2;
                    let delta_vel =
                        (well.position - *body.position).normalize_to(acceleration * dt);
                    body.velocity
                        .set(pending_updates, *body.velocity + delta_vel);
                }
            })
        });
}

#[allow(clippy::many_single_char_names)]
fn check_if_bodies_collides(body1: &Body, body2: &Body, dt: f64) -> Option<f64> {
    // r = r1 + r2
    // x = x1 - x2, y = …, z = …
    // dx = dx1 - dx2, dy = …, dz = …
    // r = ((x + dx*t)^2 + (y + dy*t)^2 + (z + dz*t)^2).sqrt()
    // 0 = (x + dx*t)^2 + (y + dy*t)^2 + (z + dz*t)^2 - r^2
    // 0 = x^2 + 2x*dx*t + (dx^2)t^2 ... - r^2
    // 0 = (dx^2 + dy^2 + dz^2)t^2 + 2(x*dx + y*dy + z*dz)t + x^2 + y^2 + z^2 - r^2
    // 0 = a*t^2 + b*t + c
    // a = dx^2 + dy^2 + dz^2
    // b = 2(x*dx + y*dy + z*dz)
    // c = x^2 + y^2 + z^2 - r^2
    let r = body1.shape.radius() + body2.shape.radius();
    if r > EPSILON {
        let rel_pos = *body1.position - *body2.position;
        let rel_vel = *body1.velocity - *body2.velocity;
        let a = rel_vel.magnitude2();
        let b = 2.0 * (rel_pos.x * rel_vel.x + rel_pos.y * rel_vel.y + rel_pos.z * rel_vel.z);
        let c = rel_pos.magnitude2() - r * r;
        // only care about the first solution (when the two spheres start touching)
        // divide by zero is fine
        let t = (-b - (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
        if t >= 0.0 && t < dt {
            return Some(t);
        }
    }
    None
}

pub fn apply_collisions(state: &State, dt: f64) {
    // TODO: sort bodies and don't compare bodies that can not touch
    state.bodies.iter().for_each(|(key1, body1)| {
        let _ = state.bodies.iter().try_for_each(|(key2, body2)| {
            if key1 == key2 {
                // We only want to process each combination of bodies once, so abort the inner loop
                // once it catches up to the outer loop
                Err(())
            } else {
                if let Some(time_until) = check_if_bodies_collides(body1, body2, dt) {
                    body1
                        .collision_handler
                        .collision(state, &Collision::new(time_until, key2));
                    body2
                        .collision_handler
                        .collision(state, &Collision::new(time_until, key1));
                }
                Ok(())
            }
        });
    });
}

pub fn apply_motion(state: &mut State, dt: f64) {
    for body in state
        .bodies
        .get_mut_without_notifying_of_change()
        .values_mut()
    {
        body.position
            .set(&state.pending_updates, *body.position + dt * *body.velocity);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod gravity_tests {
    use cgmath::Vector3;

    use super::*;
    use crate::components::Body;

    const PLANET_MASS: f64 = 5.972e+18; // mass of earth

    #[test]
    fn lone_gravity_body_is_unaffected() {
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        assert_eq!(*state.bodies[body].velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.bodies[body].velocity, velocity);
    }

    #[test]
    fn lone_gravity_body_off_origin_is_unaffected() {
        let position = Point3::new(4.0, 2.0, 6.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = state.add_body(
            Body::new()
                .with_mass(PLANET_MASS)
                .with_gravity()
                .with_position(position),
        );
        assert_eq!(*state.bodies[body].velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.bodies[body].velocity, velocity);
    }

    #[test]
    fn body_falls_towards_gravity_source() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);
        let mut state = State::new();
        state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body = state.add_body(Body::new().with_position(position));
        assert_eq!(state.bodies[body].velocity.x, 0.0);
        apply_gravity(&mut state, 1.0);
        let v = *state.bodies[body].velocity;
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
        let v_a = *state_a.bodies[body_a].velocity;

        let mut state_b = State::new();
        state_b.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body_b = state_b.add_body(Body::new().with_position(position));
        apply_gravity(&mut state_b, 0.5);
        let v_b = *state_b.bodies[body_b].velocity;

        assert!((v_a.x - (v_b.x * 2.0)).abs() < EPSILON);
    }

    #[test]
    fn falls_in_correct_direction() {
        let position = Point3::new(20.0e+3, 0.0, -20.0e+3);
        let mut state = State::new();
        state.add_body(Body::new().with_mass(PLANET_MASS).with_gravity());
        let body = state.add_body(Body::new().with_position(position));
        apply_gravity(&mut state, 1.0);
        let v = *state.bodies[body].velocity;
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
        let v = *state.bodies[body].velocity;
        assert!(v.x.abs() < EPSILON);
        assert!(v.y.abs() < EPSILON);
        assert!(v.z.abs() < EPSILON);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod collision_tests {
    use cgmath::Vector3;
    use std::sync::{Arc, RwLock};

    use super::*;
    use crate::components::{Body, Collision, CollisionHandler};
    use crate::state::BodyKey;

    struct MockController {
        collisions: Vec<Collision>,
    }

    impl MockController {
        fn new() -> Arc<RwLock<MockController>> {
            Arc::new(RwLock::new(MockController {
                collisions: Vec::new(),
            }))
        }
    }

    impl CollisionHandler for Arc<RwLock<MockController>> {
        fn collision(&self, _state: &State, collision: &Collision) {
            let mut vec = self.write().unwrap();
            vec.collisions.push(collision.clone());
        }
    }

    fn two_body_test(
        body1: Body,
        body2: Body,
    ) -> (BodyKey, BodyKey, Vec<Collision>, Vec<Collision>) {
        let mut state = State::new();
        let c1 = MockController::new();
        let c2 = MockController::new();
        let b1 = state.add_body(body1.with_collision_handler(Box::new(c1.clone())));
        let b2 = state.add_body(body2.with_collision_handler(Box::new(c2.clone())));
        apply_collisions(&state, 1.0);
        let col1 = c1.read().unwrap().collisions.clone();
        let col2 = c2.read().unwrap().collisions.clone();
        (b1, b2, col1, col2)
    }

    fn assert_do_not_collide(body1: Body, body2: Body) {
        let (_, _, col1, col2) = two_body_test(body1, body2);
        assert_eq!(col1, vec![]);
        assert_eq!(col2, vec![]);
    }

    fn assert_collides(body1: Body, body2: Body, time: f64) {
        let (b1, b2, col1, col2) = two_body_test(body1, body2);
        assert_eq!(col1.len(), 1);
        assert_eq!(col2.len(), 1);
        assert_eq!(col1[0].body, b2);
        assert_eq!(col2[0].body, b1);
        assert_eq!(col1[0].time_until, col2[0].time_until);
        assert!((col1[0].time_until - time).abs() < EPSILON);
    }

    #[test]
    fn no_collisions_for_single_point() {
        let mut state = State::new();
        let c1 = MockController::new();
        state.add_body(Body::new().with_collision_handler(Box::new(c1.clone())));
        apply_collisions(&state, 1.0);
        assert_eq!(c1.read().unwrap().collisions, vec![]);
    }

    #[test]
    fn no_collisions_for_single_sphere() {
        let mut state = State::new();
        let c1 = MockController::new();
        state.add_body(
            Body::new()
                .with_sphere_shape(1.0)
                .with_collision_handler(Box::new(c1.clone())),
        );
        apply_collisions(&state, 1.0);
        assert_eq!(c1.read().unwrap().collisions, vec![]);
    }

    #[test]
    fn no_collisions_for_single_moving_sphere() {
        let mut state = State::new();
        let c1 = MockController::new();
        state.add_body(
            Body::new()
                .with_velocity(Vector3::new(3.0, 0.5, -2.0))
                .with_sphere_shape(1.0)
                .with_collision_handler(Box::new(c1.clone())),
        );
        apply_collisions(&state, 1.0);
        assert_eq!(c1.read().unwrap().collisions, vec![]);
    }

    #[test]
    fn respects_delta_time() {
        let mut state = State::new();
        let c1 = MockController::new();
        state.add_body(
            Body::new()
                .with_sphere_shape(1.0)
                .with_collision_handler(Box::new(c1.clone())),
        );
        state.add_body(
            Body::new()
                .with_position(Point3::new(2.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 0.0)),
        );
        apply_collisions(&state, 0.25);
        assert_eq!(c1.read().unwrap().collisions, vec![]);
    }

    #[test]
    fn stationary_non_touching_spheres_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(3.0, 0.0, 0.0))
                .with_sphere_shape(1.0),
        );
    }

    #[test]
    fn stationary_non_touching_sphere_and_point_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new().with_position(Point3::new(2.0, 0.0, 0.0)),
        );
    }

    #[test]
    fn point_inside_bounding_box_does_not_collide_with_sphere() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new().with_position(Point3::new(0.9, 0.9, 0.9)),
        );
    }

    #[test]
    fn point_does_not_collide_with_sphere_when_entering_bounding_box() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(1.1, 1.1, 1.1))
                .with_velocity(Vector3::new(-0.1, -0.1, -0.1)),
        );
    }

    #[test]
    fn points_do_not_collide_even_when_they_directly_cross() {
        assert_do_not_collide(
            Body::new(),
            Body::new()
                .with_position(Point3::new(1.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 0.0)),
        );
    }

    #[test]
    fn stationary_point_inside_stationary_sphere_does_not_collide() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new().with_position(Point3::new(0.2, 0.2, 0.2)),
        );
    }

    #[test]
    fn stationary_overlapping_spheres_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(1.0, 0.0, 0.0))
                .with_sphere_shape(1.0),
        );
    }

    #[test]
    fn moving_point_collides_with_sphere() {
        assert_collides(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(1.5, 0.0, 0.0))
                .with_velocity(Vector3::new(-1.0, 0.0, 0.0)),
            0.5,
        );
    }

    #[test]
    fn moving_sphere_collides_with_stationary_sphere() {
        assert_collides(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(3.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 0.0))
                .with_sphere_shape(1.0),
            0.5,
        );
    }

    #[test]
    fn two_moving_spheres_collide() {
        assert_collides(
            Body::new()
                .with_velocity(Vector3::new(1.0, 0.0, 0.0))
                .with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(3.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-1.0, 0.0, 0.0))
                .with_sphere_shape(1.0),
            0.5,
        );
    }

    #[test]
    fn point_collides_with_stationary_sphere_even_when_it_would_make_it_out_the_back() {
        assert_collides(
            Body::new().with_sphere_shape(1.0),
            Body::new()
                .with_position(Point3::new(2.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-4.0, 0.0, 0.0)),
            0.25,
        );
    }

    #[test]
    fn moving_spheres_in_irregular_places_collide() {
        // 3 = sqrt((3 - 3t)^2 + (2 + 0t)^2  + (0.5 + 1t)^2)
        // ^^ paste that into WolframAlpha because fuck Algrebra II
        // t = 0.304564
        assert_collides(
            Body::new()
                .with_position(Point3::new(0.0, -1.0, -0.5))
                .with_velocity(Vector3::new(1.0, 0.0, 0.0))
                .with_sphere_shape(2.0),
            Body::new()
                .with_position(Point3::new(3.0, 1.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 1.0))
                .with_sphere_shape(1.0),
            0.304_564,
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod motion_tests {
    use cgmath::Vector3;

    use super::*;
    use crate::components::Body;

    #[test]
    fn no_motion_if_zero_velocity() {
        let mut state = State::new();
        let body = state.add_body(Body::new());
        assert_eq!(*state.bodies[body].position, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(*state.bodies[body].velocity, Vector3::new(0.0, 0.0, 0.0));
        apply_motion(&mut state, 1.0);
        assert_eq!(*state.bodies[body].position, Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn moves_bodies_by_velocity_amount() {
        let mut state = State::new();
        let body1 = state.add_body(
            Body::new()
                .with_position(Point3::new(-1.0, 4.0, 0.0))
                .with_velocity(Vector3::new(1.0, 0.0, 2.0)),
        );
        let body2 = state.add_body(Body::new().with_velocity(Vector3::new(0.0, 0.5, 0.0)));
        apply_motion(&mut state, 1.0);
        assert_eq!(*state.bodies[body1].position, Point3::new(0.0, 4.0, 2.0));
        assert_eq!(*state.bodies[body2].position, Point3::new(0.0, 0.5, 0.0));
    }

    #[test]
    fn respects_dt() {
        let mut state = State::new();
        let body = state.add_body(Body::new().with_velocity(Vector3::new(4.0, 0.0, 0.0)));
        apply_motion(&mut state, 0.5);
        assert_eq!(*state.bodies[body].position, Point3::new(2.0, 0.0, 0.0));
    }
}
