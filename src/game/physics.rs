use super::*;

/// G = 6.67430e-11 N * m^2 / kg^2
/// N is in kg * m * s^-2
/// That means that converting to our units (km and mt) we get…
const GRAVITATIONAL_CONSTANT: f64 = 6.67430e-17;

/// Applies the force of gravity to bodies' velocities
pub fn apply_gravity(state: &mut State, dt: f64) {
    // we can't access the body (and thus the position) of a gravity well while we are mutating the
    // position of bodies, so we collect all the info we need into a local vec (which should be
    // good for performence as well)
    struct GravityWell {
        position: Point3<f64>,
        mass: f64,
    };
    let wells: Vec<GravityWell> = state
        .components_iter::<GravityBody>()
        .map(|(entity, _)| {
            // TODO: error handing on body not in bodies
            let body = state
                .component::<Body>(entity)
                .expect("GravityBody does not have a body");
            GravityWell {
                position: *body.position,
                mass: *body.mass,
            }
        })
        .collect();
    let iter = state.components_iter_mut::<Body>();
    iter.for_each(|(_, body)| {
        wells.iter().for_each(|well| {
            let distance2 = well.position.distance2(*body.position);
            if distance2 > EPSILON {
                let acceleration = GRAVITATIONAL_CONSTANT * well.mass / distance2;
                let delta_vel = (well.position - *body.position).normalize_to(acceleration * dt);
                body.velocity.set(*body.velocity + delta_vel);
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

/// Handles body collisions
pub fn apply_collisions(state: &State, dt: f64) {
    // TODO: sort bodies and don't compare bodies that can not touch
    state.components_iter::<Body>().for_each(|(key1, body1)| {
        let _ = state
            .components_iter::<Body>()
            .try_for_each(|(key2, body2)| {
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

/// Applies thrust of all ships to their velocity
pub fn apply_acceleration(state: &mut State, dt: f64) {
    // Collecting keys into a vec is wastefull, but seems to be the only way currently
    // TODO: improve the ECS so this can be done in one pass
    let ships: Vec<EntityKey> = state.components_iter::<Ship>().map(|(e, _)| e).collect();
    for e in ships {
        let thrust = *state.component::<Ship>(e).unwrap().acceleration;
        let vel = &mut state.component_mut::<Body>(e).unwrap().velocity;
        vel.set(**vel + thrust * dt);
    }
}

/// Applies velocity of all bodies to their position
pub fn apply_motion(state: &mut State, dt: f64) {
    let iter = state.components_iter_mut::<Body>();
    for (_, body) in iter {
        body.position.set(*body.position + dt * *body.velocity);
        //info!("position: {:?}", *body.position);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod gravity_tests {
    use super::*;

    const EARTH_MASS: f64 = 5.972e+21; // mass of earth
    const EARTH_RADIUS: f64 = 6368.0; // radius of earth

    fn create_body_entity(state: &mut State, body: Body, gravity: bool) -> EntityKey {
        let entity = state.create_entity();
        state.install_component(entity, body);
        if gravity {
            state.install_component(entity, GravityBody);
        }
        entity
    }

    #[test]
    fn lone_gravity_body_is_unaffected() {
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = create_body_entity(&mut state, Body::new().with_mass(EARTH_MASS), true);
        assert_eq!(*state.component::<Body>(body).unwrap().velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.component::<Body>(body).unwrap().velocity, velocity);
    }

    #[test]
    fn lone_gravity_body_off_origin_is_unaffected() {
        let position = Point3::new(4.0, 2.0, 6.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = create_body_entity(
            &mut state,
            Body::new().with_mass(EARTH_MASS).with_position(position),
            true,
        );
        assert_eq!(*state.component::<Body>(body).unwrap().velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.component::<Body>(body).unwrap().velocity, velocity);
    }

    #[test]
    fn body_falls_towards_gravity_source() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);
        let mut state = State::new();
        let _ = create_body_entity(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body_entity(&mut state, Body::new().with_position(position), false);
        assert_eq!(state.component::<Body>(body).unwrap().velocity.x, 0.0);
        apply_gravity(&mut state, 1.0);
        let v = *state.component::<Body>(body).unwrap().velocity;
        assert!(v.x < -EPSILON);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
    }

    #[test]
    fn acceleration_proportional_to_dt() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);

        let mut state_a = State::new();
        let _ = create_body_entity(&mut state_a, Body::new().with_mass(EARTH_MASS), true);
        let body_a = create_body_entity(&mut state_a, Body::new().with_position(position), false);
        apply_gravity(&mut state_a, 1.0);
        let v_a = *state_a.component::<Body>(body_a).unwrap().velocity;

        let mut state_b = State::new();
        let _ = create_body_entity(&mut state_b, Body::new().with_mass(EARTH_MASS), true);
        let body_b = create_body_entity(&mut state_b, Body::new().with_position(position), false);
        apply_gravity(&mut state_b, 0.5);
        let v_b = *state_b.component::<Body>(body_b).unwrap().velocity;

        assert!((v_a.x - (v_b.x * 2.0)).abs() < EPSILON);
    }

    #[test]
    fn falls_in_correct_direction() {
        let position = Point3::new(20.0e+3, 0.0, -20.0e+3);
        let mut state = State::new();
        let _ = create_body_entity(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body_entity(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.component::<Body>(body).unwrap().velocity;
        assert!(v.x < -EPSILON);
        assert!(v.y.abs() < EPSILON);
        assert!(v.z > EPSILON);
        assert!((v.x + v.z).abs() < EPSILON);
    }

    #[test]
    fn multiple_wells_cancel_each_other_out() {
        let position = Point3::new(-20.0e+3, 27.5, 154.0);
        let mut state = State::new();
        let _ = create_body_entity(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let _ = create_body_entity(
            &mut state,
            Body::new()
                .with_mass(EARTH_MASS)
                .with_position(position * 2.0),
            true,
        );
        let body = create_body_entity(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.component::<Body>(body).unwrap().velocity;
        assert!(v.x.abs() < EPSILON);
        assert!(v.y.abs() < EPSILON);
        assert!(v.z.abs() < EPSILON);
    }

    #[test]
    fn accel_on_earth_is_about_right() {
        let position = Point3::new(-EARTH_RADIUS, 0.0, 0.0);
        let mut state = State::new();
        let _ = create_body_entity(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body_entity(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.component::<Body>(body).unwrap().velocity;
        assert!(v.y.abs() < EPSILON);
        assert!(v.z.abs() < EPSILON);
        // When converted to meters/s, should be the well known value 9.81 (measured accel due to
        // gravity on earth's surface). Because of various factors (centripetal force, earth's mass
        // being distributed throughout the planet, etc) it wont be exact.
        let acce_m_per_s = v.x * 1000.0;
        println!("{}", acce_m_per_s);
        assert!(acce_m_per_s > 9.7);
        assert!(acce_m_per_s < 9.9);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod collision_tests {
    use super::*;

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
    ) -> (EntityKey, EntityKey, Vec<Collision>, Vec<Collision>) {
        let mut state = State::new();
        let c1 = MockController::new();
        let c2 = MockController::new();
        let b1 = state.create_entity();
        state.install_component(b1, body1.with_collision_handler(Box::new(c1.clone())));
        let b2 = state.create_entity();
        state.install_component(b2, body2.with_collision_handler(Box::new(c2.clone())));
        apply_collisions(&state, 1.0);
        let col1 = c1.read().unwrap().collisions.clone();
        let col2 = c2.read().unwrap().collisions.clone();
        (b1, b2, col1, col2)
    }

    fn create_body_entity(state: &mut State, body: Body) {
        let entity = state.create_entity();
        state.install_component(entity, body);
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
        create_body_entity(
            &mut state,
            Body::new().with_collision_handler(Box::new(c1.clone())),
        );
        apply_collisions(&state, 1.0);
        assert_eq!(c1.read().unwrap().collisions, vec![]);
    }

    #[test]
    fn no_collisions_for_single_sphere() {
        let mut state = State::new();
        let c1 = MockController::new();
        create_body_entity(
            &mut state,
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
        create_body_entity(
            &mut state,
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
        create_body_entity(
            &mut state,
            Body::new()
                .with_sphere_shape(1.0)
                .with_collision_handler(Box::new(c1.clone())),
        );
        create_body_entity(
            &mut state,
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
    use super::*;

    fn create_body_entity(state: &mut State, body: Body) -> EntityKey {
        let entity = state.create_entity();
        state.install_component(entity, body);
        entity
    }

    #[test]
    fn no_motion_if_zero_velocity() {
        let mut state = State::new();
        let body = create_body_entity(&mut state, Body::new());
        assert_eq!(
            *state.component::<Body>(body).unwrap().position,
            Point3::new(0.0, 0.0, 0.0)
        );
        assert_eq!(
            *state.component::<Body>(body).unwrap().velocity,
            Vector3::new(0.0, 0.0, 0.0)
        );
        apply_motion(&mut state, 1.0);
        assert_eq!(
            *state.component::<Body>(body).unwrap().position,
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn moves_bodies_by_velocity_amount() {
        let mut state = State::new();
        let body1 = create_body_entity(
            &mut state,
            Body::new()
                .with_position(Point3::new(-1.0, 4.0, 0.0))
                .with_velocity(Vector3::new(1.0, 0.0, 2.0)),
        );
        let body2 = create_body_entity(
            &mut state,
            Body::new().with_velocity(Vector3::new(0.0, 0.5, 0.0)),
        );
        apply_motion(&mut state, 1.0);
        assert_eq!(
            *state.component::<Body>(body1).unwrap().position,
            Point3::new(0.0, 4.0, 2.0)
        );
        assert_eq!(
            *state.component::<Body>(body2).unwrap().position,
            Point3::new(0.0, 0.5, 0.0)
        );
    }

    #[test]
    fn respects_dt() {
        let mut state = State::new();
        let body = create_body_entity(
            &mut state,
            Body::new().with_velocity(Vector3::new(4.0, 0.0, 0.0)),
        );
        apply_motion(&mut state, 0.5);
        assert_eq!(
            *state.component::<Body>(body).unwrap().position,
            Point3::new(2.0, 0.0, 0.0)
        );
    }
}
