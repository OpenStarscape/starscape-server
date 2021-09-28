use super::*;

const DYNAMIC_TEST_RELATIVE_DELTA_TIME: f64 = 0.0001;
const DYNAMIC_TEST_POSITION_RELATIVE_THRESHOLD: f64 = 0.02;
const DYNAMIC_TEST_VELOCITY_RELATIVE_THRESHOLD: f64 = 0.03;
const ORBIT_EPSILON: f64 = 1.0e-8;
const ORBIT_MAX_ULPS: u32 = 12;

/// Modulo that returns a positive (or very slightly negative) result,
/// useful for canonicalizing things like angles
fn positiveish_mod(value: f64, modulo: f64) -> f64 {
    value - ((value + ORBIT_EPSILON) / modulo).floor() * modulo
}

fn canonicalize_orbit(orbit: &OrbitData) -> OrbitData {
    let mut orbit = *orbit;
    if ulps_eq!(orbit.inclination, 0.0) {
        // flat orbit. ascending_node doesn't matter.
        orbit.periapsis += orbit.ascending_node;
        orbit.ascending_node = 0.0;
    }
    if ulps_eq!(orbit.semi_major, orbit.semi_minor) {
        // circular orbit. periapsis doesn't matter.
        orbit.base_time -= orbit.periapsis * orbit.period_time / TAU;
        orbit.periapsis = 0.0;
    }
    // Make sure base_time is always between 0 and period_time
    orbit.base_time = positiveish_mod(orbit.base_time, orbit.period_time);
    orbit.inclination = positiveish_mod(orbit.inclination, TAU);
    orbit.ascending_node = positiveish_mod(orbit.ascending_node, TAU);
    orbit.periapsis = positiveish_mod(orbit.periapsis, TAU);
    orbit
}

fn orbit_params_as_slice(orbit: &OrbitData) -> [f64; 7] {
    [
        orbit.semi_major,
        orbit.semi_minor,
        orbit.inclination,
        orbit.ascending_node,
        orbit.periapsis,
        orbit.base_time,
        orbit.period_time,
    ]
}

fn orbits_eq(a: &OrbitData, b: &OrbitData) -> bool {
    let a_params = orbit_params_as_slice(&a);
    let b_params = orbit_params_as_slice(&b);
    for i in 0..a_params.len() {
        if ulps_ne!(
            a_params[i],
            b_params[i],
            max_ulps = ORBIT_MAX_ULPS,
            epsilon = ORBIT_EPSILON
        ) {
            return false;
        }
    }
    a.parent == b.parent
}

pub fn static_orbit_test(
    mut orbit: OrbitData,
    grav_param: f64,
    at_time: f64,
    position: Point3<f64>,
    velocity: Vector3<f64>,
    position_offset: Point3<f64>,
    velocity_offset: Vector3<f64>,
) {
    let mut state = State::new();
    state.increment_physics(at_time);
    let parent_mass = grav_param / GRAVITATIONAL_CONSTANT;
    orbit.parent = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(position_offset)
        .with_velocity(velocity_offset)
        .with_mass(parent_mass)
        .with_name("parent".to_string())
        .install(&mut state, orbit.parent);
    let body = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(position_offset + position.to_vec())
        .with_velocity(velocity_offset + velocity)
        .with_mass(parent_mass / 10.0)
        .with_name("body".to_string())
        .install(&mut state, body);
    state
        .component_mut::<Body>(body)
        .unwrap()
        .gravity_parent
        .set(orbit.parent);
    let orbit_value = state.get_property(ConnectionKey::null(), body, "orbit");
    match orbit_value {
        Ok(value) => {
            if value.is_null() {
                panic!("orbit is null instead of {:#?}", orbit);
            }
            match RequestResult::<OrbitData>::from(value) {
                Ok(actual) => {
                    let expected = canonicalize_orbit(&orbit);
                    let actual = canonicalize_orbit(&actual);
                    if !orbits_eq(&actual, &expected) {
                        panic!("expected {:#?}, got {:#?}", expected, actual);
                    }
                }
                Err(e) => panic!("Orbit property produced invalid type: {}", e),
            }
        }
        Err(err) => panic!("getting orbit value produced error: {}", err),
    }
}

/// The dynamic tests don't touch the orbit conduit at all. Instead they start the body at
/// the periapsis, run physics until at_time and check if it ends up in the right position
pub fn dynamic_orbit_test(
    orbit: OrbitData,
    grav_param: f64,
    at_time: f64,
    position: Point3<f64>,
    velocity: Vector3<f64>,
) {
    let run_time = positiveish_mod(at_time - orbit.base_time, orbit.period_time);
    let rotation = Basis3::from_angle_z(Rad(orbit.ascending_node))
        * Basis3::from_angle_x(Rad(orbit.inclination))
        * Basis3::from_angle_z(Rad(orbit.periapsis));
    let radius_at_periapsis =
        orbit.semi_major - (orbit.semi_major.powf(2.0) - orbit.semi_minor.powf(2.0)).sqrt();
    let start_position = rotation.rotate_point(Point3::new(radius_at_periapsis, 0.0, 0.0));
    let start_speed = (grav_param * (2.0 / radius_at_periapsis - 1.0 / orbit.semi_major)).sqrt();
    let start_velocity = rotation.rotate_vector(Vector3::new(0.0, start_speed, 0.0));
    let mut state = State::new();
    let parent_mass = grav_param / GRAVITATIONAL_CONSTANT;
    let parent = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_mass(parent_mass)
        .with_name("parent".to_string())
        .install(&mut state, parent);
    let body = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(start_position)
        .with_velocity(start_velocity)
        .with_mass(parent_mass / 100000.0)
        .with_name("body".to_string())
        .install(&mut state, body);
    let delta = orbit.period_time * DYNAMIC_TEST_RELATIVE_DELTA_TIME;
    while state.time() < run_time {
        apply_gravity(&mut state, delta);
        apply_motion(&mut state, delta);
        state.increment_physics(delta);
    }
    let actual_position = *state.component::<Body>(body).unwrap().position;
    let actual_velocity = *state.component::<Body>(body).unwrap().velocity;
    let position_delta = actual_position.distance(position);
    let velocity_delta = actual_velocity.distance(velocity);
    let position_threshold = orbit.semi_major * DYNAMIC_TEST_POSITION_RELATIVE_THRESHOLD;
    let velocity_threshold =
        orbit.semi_major / orbit.period_time * DYNAMIC_TEST_VELOCITY_RELATIVE_THRESHOLD;
    if position_delta > position_threshold {
        panic!(
            "expected result position {:#?}, got {:#?} (distance: {})",
            position, actual_position, position_delta
        );
    }
    if velocity_delta > velocity_threshold {
        panic!(
            "expected result velocity {:#?}, got {:#?} (difference: {})",
            velocity, actual_velocity, velocity_delta
        );
    }
}
