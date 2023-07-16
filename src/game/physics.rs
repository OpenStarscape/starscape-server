use super::*;

/// G = 6.67430e-11 N * m^2 / kg^2
/// N is in kg * m * s^-2
/// That means that converting to our units (km and mt) we get…
pub const GRAVITATIONAL_CONSTANT: f64 = 6.67430e-17;

/// Applies the force of gravity to bodies' velocities
pub fn apply_gravity(state: &mut State, dt: f64) {
    // we can't access the body (and thus the position) of a gravity well while we are mutating the
    // position of bodies, so we collect all the info we need into a local vec (which should be
    // good for performence as well)
    struct GravityWell {
        id: Id<Body>,
        position: Point3<f64>,
        velocity: Vector3<f64>,
        mass: f64,
        /// radius of the sphere-of-influence squared
        sphere_of_influence2: f64,
    }
    let mut wells: Vec<GravityWell> = state
        .iter::<Body>()
        .filter(|(_id, body)| body.is_gravity_well())
        .map(|(id, body)| GravityWell {
            id,
            position: *body.position,
            velocity: *body.velocity,
            mass: *body.mass,
            sphere_of_influence2: 0.0,
        })
        .collect();
    // For the sphere of influence calculation, we need to look at gravity wells in descending order
    wells.sort_unstable_by(|a, b| {
        b.mass
            .partial_cmp(&a.mass)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if !wells.is_empty() {
        // This will be the most massive object, presumably the sun
        wells[0].sphere_of_influence2 = f64::INFINITY;
    }
    // Now calculate the sphere of influence of each body. To do this, we need to know the body's parent. But wait!
    // isn't figuring out parents the reason we're calculating sphere of influence in the first place? Uh, yeah, so we
    // have to get fancy. This is where the sorting comes in. We go through bodies in order of descending mass. For each
    // body we only consider bodies we've already done (so bodies more massive than the current one). We find the last
    // massive body that the current body is in the sphere of influence of. That is considered the parent, and lets us
    // calculate the current body's sphere of influence.
    for current in 1..wells.len() {
        let current_position = wells[current].position;
        for parent in (0..current).rev() {
            // Get the distance², which is faster than normal distance and possibly all we need
            let distance2 = current_position.distance2(wells[parent].position);
            if distance2 <= wells[parent].sphere_of_influence2 {
                // Should we set gravity_parent here? We could, but it will also be set below so don't bother
                // Distance to the parent
                let distance = distance2.sqrt();
                let velocity2 = wells[current].velocity.distance2(wells[parent].velocity);
                let g_times_parent_mass = GRAVITATIONAL_CONSTANT * wells[parent].mass;
                // Semi-major axis of the current body's orbit around the parent
                let semi_major = distance * g_times_parent_mass
                    / (2.0 * g_times_parent_mass - distance * velocity2);
                // Sphere of influence (approximate)
                let sphere_of_influence =
                    semi_major * (wells[current].mass / wells[parent].mass).powf(2.0 / 5.0);
                wells[current].sphere_of_influence2 = sphere_of_influence * sphere_of_influence;
            }
        }
    }
    let iter = state.iter_mut::<Body>();
    iter.for_each(|(id, body)| {
        let (grav_parent, _grav_parent_mass) = wells.iter().fold(
            (Id::null(), f64::INFINITY),
            |(grav_parent, grav_parent_mass), well| {
                if well.id != id {
                    // Get the distance², which is faster than normal distance and all we need
                    let distance2 = well.position.distance2(*body.position);
                    // Acceleration due to gravity follows the inverse square law
                    let acceleration = GRAVITATIONAL_CONSTANT * well.mass / distance2;
                    // Change in velocity is previously calculated acceleration towards the well
                    let delta_vel =
                        (well.position - *body.position).normalize_to(acceleration * dt);
                    // Apply delta-velocity to the body
                    body.velocity.set(*body.velocity + delta_vel);
                    // Now we check if if the well is a candidate to be this body's gravity parent. To be one it must:
                    // - Be less massive than the current candidate
                    // - Be more massive than the body
                    // - Have a sphere of influence that includes the body
                    if well.mass < grav_parent_mass
                        && well.mass >= *body.mass
                        && distance2 <= well.sphere_of_influence2
                    {
                        return (well.id, well.mass);
                    }
                }
                (grav_parent, grav_parent_mass)
            },
        );
        body.gravity_parent.set(grav_parent);
    });
}

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

struct Collision {
    #[allow(dead_code)]
    time_until: f64,
    us: Id<Body>,
    them: Id<Body>,
}

fn handle_collision(state: &mut State, collision: &Collision) -> Result<(), Box<dyn Error>> {
    let our_body = state.get(collision.us)?;
    let other_body = state.get(collision.them)?;
    let rel_vel = *other_body.velocity - *our_body.velocity;
    let mass_ratio = *other_body.mass / (*our_body.mass + *other_body.mass);
    let vel_change = rel_vel * mass_ratio;
    let max_vel_change = our_body.shape.radius() * 2.5;
    if vel_change.magnitude2() > max_vel_change * max_vel_change {
        state.remove(collision.us)?;
    } else {
        *state.get_mut(collision.us)?.velocity.get_mut() += vel_change;
    }
    Ok(())
}

fn find_collisions(state: &State, dt: f64) -> Vec<Collision> {
    // TODO: sort bodies and don't compare bodies that can not touch
    let mut collisions = Vec::new();
    state.iter().for_each(|(id1, body1)| {
        let _ = state.iter().try_for_each(|(id2, body2)| {
            if id1 == id2 {
                // We only want to process each combination of bodies once, so abort the inner loop
                // once it catches up to the outer loop
                Err(())
            } else {
                if let Some(time_until) = check_if_bodies_collides(body1, body2, dt) {
                    collisions.push(Collision {
                        time_until,
                        us: id1,
                        them: id2,
                    });
                    collisions.push(Collision {
                        time_until,
                        us: id2,
                        them: id1,
                    });
                }
                Ok(())
            }
        });
    });
    collisions
}

/// Handles body collisions
pub fn apply_collisions(state: &mut State, dt: f64) {
    let collisions = find_collisions(state, dt);
    for collision in &collisions {
        handle_collision(state, collision).or_log_warn("handling collision");
    }
}

/// Applies thrust of all ships to their velocity
pub fn apply_acceleration(state: &mut State, dt: f64) {
    // TODO: make a more effecient way of iterating through all ships
    for (_id, body) in state.iter_mut::<Body>() {
        if let BodyClass::Ship(ship) = &body.class {
            let thrust = *ship.acceleration;
            let vel = *body.velocity;
            body.velocity.set(vel + thrust * dt);
        }
    }
}

/// Applies velocity of all bodies to their position
pub fn apply_motion(state: &mut State, dt: f64) {
    for (_id, body) in state.iter_mut::<Body>() {
        let pos = *body.position;
        let vel = *body.velocity;
        body.position.set(pos + dt * vel);
    }
}

#[cfg(test)]
mod gravity_tests {
    use super::*;

    const EARTH_MASS: f64 = 5.972e+21; // mass of earth
    const EARTH_RADIUS: f64 = 6368.0; // radius of earth

    fn create_body(state: &mut State, body: Body, _gravity: bool) -> Id<Body> {
        let id = state.add_without_object(body);
        // TODO
        /*
        if gravity {
            state.install_component(entity, GravityBody);
        }
        */
        id
    }

    #[test]
    fn lone_gravity_body_is_unaffected() {
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        assert_ulps_eq!(*state.get(body).unwrap().velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_ulps_eq!(*state.get(body).unwrap().velocity, velocity);
    }

    #[test]
    fn lone_gravity_body_off_origin_is_unaffected() {
        let position = Point3::new(4.0, 2.0, 6.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);
        let mut state = State::new();
        let body = create_body(
            &mut state,
            Body::new().with_mass(EARTH_MASS).with_position(position),
            true,
        );
        assert_ulps_eq!(*state.get(body).unwrap().velocity, velocity);
        apply_gravity(&mut state, 1.0);
        assert_ulps_eq!(*state.get(body).unwrap().velocity, velocity);
    }

    #[test]
    fn body_falls_towards_gravity_source() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);
        let mut state = State::new();
        let _ = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body(&mut state, Body::new().with_position(position), false);
        assert_ulps_eq!(state.get(body).unwrap().velocity.x, 0.0);
        apply_gravity(&mut state, 1.0);
        let v = *state.get(body).unwrap().velocity;
        assert!(v.x < -EPSILON);
        assert_ulps_eq!(v.y, 0.0);
        assert_ulps_eq!(v.z, 0.0);
    }

    #[test]
    fn acceleration_proportional_to_dt() {
        let position = Point3::new(20.0e+3, 0.0, 0.0);

        let mut state_a = State::new();
        let _ = create_body(&mut state_a, Body::new().with_mass(EARTH_MASS), true);
        let body_a = create_body(&mut state_a, Body::new().with_position(position), false);
        apply_gravity(&mut state_a, 1.0);
        let v_a = *state_a.get(body_a).unwrap().velocity;

        let mut state_b = State::new();
        let _ = create_body(&mut state_b, Body::new().with_mass(EARTH_MASS), true);
        let body_b = create_body(&mut state_b, Body::new().with_position(position), false);
        apply_gravity(&mut state_b, 0.5);
        let v_b = *state_b.get(body_b).unwrap().velocity;

        assert_ulps_eq!(v_a.x - (v_b.x * 2.0), 0.0);
    }

    #[test]
    fn falls_in_correct_direction() {
        let position = Point3::new(20.0e+3, 0.0, -20.0e+3);
        let mut state = State::new();
        let _ = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.get(body).unwrap().velocity;
        assert!(v.x < -EPSILON);
        assert_ulps_eq!(v.y, 0.0);
        assert!(v.z > EPSILON);
        assert_ulps_eq!(v.x + v.z, 0.0);
    }

    #[test]
    fn multiple_wells_cancel_each_other_out() {
        let position = Point3::new(-20.0e+3, 27.5, 154.0);
        let mut state = State::new();
        let _ = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let _ = create_body(
            &mut state,
            Body::new()
                .with_mass(EARTH_MASS)
                .with_position(position * 2.0),
            true,
        );
        let body = create_body(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.get(body).unwrap().velocity;
        assert_ulps_eq!(v, Vector3::zero());
    }

    #[test]
    fn gravity_parent_for_two_body_system() {
        let position = Point3::new(-20.0e+3, 27.5, 154.0);
        let velocity = Vector3::new(0.0, 6.0, 0.0);
        let mut state = State::new();
        let planet = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body(
            &mut state,
            Body::new().with_position(position).with_velocity(velocity),
            false,
        );
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.get(body).unwrap().gravity_parent, planet);
        assert_eq!(*state.get(planet).unwrap().gravity_parent, Id::null());
    }

    #[test]
    fn gravity_parent_for_three_body_system() {
        let position_a = Point3::new(-2.0e+6, 27.5, 154.0);
        let position_b = position_a + Vector3::new(100.0, 0.0, 0.0);
        let velocity = Vector3::new(0.0, 1.0, 0.0);
        let mut state = State::new();
        let sun = create_body(&mut state, Body::new().with_mass(EARTH_MASS * 100.0), true);
        let planet = create_body(
            &mut state,
            Body::new()
                .with_position(position_a)
                .with_velocity(velocity)
                .with_mass(EARTH_MASS),
            true,
        );
        let body = create_body(&mut state, Body::new().with_position(position_b), false);
        apply_gravity(&mut state, 1.0);
        assert_eq!(*state.get(sun).unwrap().gravity_parent, Id::null());
        assert_eq!(*state.get(planet).unwrap().gravity_parent, sun);
        assert_eq!(*state.get(body).unwrap().gravity_parent, planet);
    }

    #[test]
    fn accel_on_earth_is_about_right() {
        let position = Point3::new(-EARTH_RADIUS, 0.0, 0.0);
        let mut state = State::new();
        let _ = create_body(&mut state, Body::new().with_mass(EARTH_MASS), true);
        let body = create_body(&mut state, Body::new().with_position(position), false);
        apply_gravity(&mut state, 1.0);
        let v = *state.get(body).unwrap().velocity;
        assert_ulps_eq!(v.y, 0.0);
        assert_ulps_eq!(v.z, 0.0);
        // When converted to meters/s, should be the well known value 9.81 (measured accel due to
        // gravity on earth's surface). Because of various factors (centripetal force, earth's mass
        // being distributed throughout the planet, etc) it wont be exact.
        let acce_m_per_s = v.x * 1000.0;
        assert!(acce_m_per_s > 9.7);
        assert!(acce_m_per_s < 9.9);
    }
}

#[cfg(test)]
mod collision_tests {
    use super::*;

    fn two_body_test(body1: Body, body2: Body) -> (Id<Body>, Id<Body>, Vec<Collision>) {
        let mut state = State::new();
        let b1 = body1.install(&mut state);
        let b2 = body2.install(&mut state);
        let collisions = find_collisions(&state, 1.0);
        (b1, b2, collisions)
    }

    fn assert_do_not_collide(body1: Body, body2: Body) {
        let (_, _, col) = two_body_test(body1, body2);
        assert_eq!(col.len(), 0);
    }

    fn assert_collides(body1: Body, body2: Body, time: f64) {
        let (b1, b2, col) = two_body_test(body1, body2);
        assert_eq!(col.len(), 2);
        assert!(col.iter().any(|col| col.us == b1 && col.them == b2));
        assert!(col.iter().any(|col| col.us == b2 && col.them == b1));
        assert_ulps_eq!(col[0].time_until, col[1].time_until, epsilon = 0.0001);
        assert_ulps_eq!(col[0].time_until, time, epsilon = 0.0001);
    }

    fn assert_signle_does_not_collide(body: Body) {
        let mut state = State::new();
        state.add_without_object(body);
        let cols = find_collisions(&state, 1.0);
        assert_eq!(cols.len(), 0);
    }

    #[test]
    fn no_collisions_for_single_point() {
        assert_signle_does_not_collide(Body::new());
    }

    #[test]
    fn no_collisions_for_single_sphere() {
        assert_signle_does_not_collide(Body::new().with_shape(Shape::from_radius(1.0).unwrap()));
    }

    #[test]
    fn no_collisions_for_single_moving_sphere() {
        assert_signle_does_not_collide(
            Body::new()
                .with_velocity(Vector3::new(3.0, 0.5, -2.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
        );
    }

    #[test]
    fn respects_delta_time() {
        let mut state = State::new();
        state.add_without_object(Body::new().with_shape(Shape::from_radius(1.0).unwrap()));
        state.add_without_object(
            Body::new()
                .with_position(Point3::new(2.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 0.0)),
        );
        let collisions = find_collisions(&state, 0.25);
        assert_eq!(collisions.len(), 0);
    }

    #[test]
    fn stationary_non_touching_spheres_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new()
                .with_position(Point3::new(3.0, 0.0, 0.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
        );
    }

    #[test]
    fn stationary_non_touching_sphere_and_point_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new().with_position(Point3::new(2.0, 0.0, 0.0)),
        );
    }

    #[test]
    fn point_inside_bounding_box_does_not_collide_with_sphere() {
        assert_do_not_collide(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new().with_position(Point3::new(0.9, 0.9, 0.9)),
        );
    }

    #[test]
    fn point_does_not_collide_with_sphere_when_entering_bounding_box() {
        assert_do_not_collide(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
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
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new().with_position(Point3::new(0.2, 0.2, 0.2)),
        );
    }

    #[test]
    fn stationary_overlapping_spheres_do_not_collide() {
        assert_do_not_collide(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new()
                .with_position(Point3::new(1.0, 0.0, 0.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
        );
    }

    #[test]
    fn moving_point_collides_with_sphere() {
        assert_collides(
            Body::new().with_shape(Shape::from_radius(2.0).unwrap()),
            Body::new()
                .with_position(Point3::new(2.5, 0.0, 0.0))
                .with_velocity(Vector3::new(-1.0, 0.0, 0.0)),
            0.5,
        );
    }

    #[test]
    fn moving_sphere_collides_with_stationary_sphere() {
        assert_collides(
            Body::new().with_shape(Shape::from_radius(0.3).unwrap()),
            Body::new()
                .with_position(Point3::new(1.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-1.0, 0.0, 0.0))
                .with_shape(Shape::from_radius(0.2).unwrap()),
            0.5,
        );
    }

    #[test]
    fn two_moving_spheres_collide() {
        assert_collides(
            Body::new()
                .with_velocity(Vector3::new(1.0, 0.0, 0.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
            Body::new()
                .with_position(Point3::new(3.0, 0.0, 0.0))
                .with_velocity(Vector3::new(-1.0, 0.0, 0.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
            0.5,
        );
    }

    #[test]
    fn point_collides_with_stationary_sphere_even_when_it_would_make_it_out_the_back() {
        assert_collides(
            Body::new().with_shape(Shape::from_radius(1.0).unwrap()),
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
                .with_shape(Shape::from_radius(2.0).unwrap()),
            Body::new()
                .with_position(Point3::new(3.0, 1.0, 0.0))
                .with_velocity(Vector3::new(-2.0, 0.0, 1.0))
                .with_shape(Shape::from_radius(1.0).unwrap()),
            0.304_564,
        );
    }
}

#[cfg(test)]
mod motion_tests {
    use super::*;

    fn create_body(state: &mut State, body: Body) -> Id<Body> {
        body.install(state)
    }

    #[test]
    fn no_motion_if_zero_velocity() {
        let mut state = State::new();
        let body = create_body(&mut state, Body::new());
        assert_ulps_eq!(
            *state.get(body).unwrap().position,
            Point3::new(0.0, 0.0, 0.0)
        );
        assert_ulps_eq!(
            *state.get(body).unwrap().velocity,
            Vector3::new(0.0, 0.0, 0.0)
        );
        apply_motion(&mut state, 1.0);
        assert_ulps_eq!(
            *state.get(body).unwrap().position,
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn moves_bodies_by_velocity_amount() {
        let mut state = State::new();
        let body1 = create_body(
            &mut state,
            Body::new()
                .with_position(Point3::new(-1.0, 4.0, 0.0))
                .with_velocity(Vector3::new(1.0, 0.0, 2.0)),
        );
        let body2 = create_body(
            &mut state,
            Body::new().with_velocity(Vector3::new(0.0, 0.5, 0.0)),
        );
        apply_motion(&mut state, 1.0);
        assert_ulps_eq!(
            *state.get(body1).unwrap().position,
            Point3::new(0.0, 4.0, 2.0)
        );
        assert_ulps_eq!(
            *state.get(body2).unwrap().position,
            Point3::new(0.0, 0.5, 0.0)
        );
    }

    #[test]
    fn respects_dt() {
        let mut state = State::new();
        let body = create_body(
            &mut state,
            Body::new().with_velocity(Vector3::new(4.0, 0.0, 0.0)),
        );
        apply_motion(&mut state, 0.5);
        assert_ulps_eq!(
            *state.get(body).unwrap().position,
            Point3::new(2.0, 0.0, 0.0)
        );
    }
}
