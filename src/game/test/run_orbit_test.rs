use super::*;

/// Modulo that always returns a positive result, useful for canonicalizing things like angles
fn positive_mod(value: f64, modulo: f64) -> f64 {
    value - (value / modulo).floor() * modulo
}

fn canonicalize_orbit(orbit: &OrbitData) -> OrbitData {
    let mut orbit = *orbit;
    assert!(orbit.base_time >= 0.0);
    assert!(orbit.base_time <= orbit.period_time);
    // TODO: better floating point comparison
    if orbit.inclination == 0.0 {
        // flat orbit. ascending_node doesn't matter.
        orbit.periapsis += orbit.ascending_node;
        orbit.ascending_node = 0.0;
    }
    // TODO: better floating point comparison
    if orbit.semi_major == orbit.semi_minor {
        // circular orbit. periapsis doesn't matter.
        orbit.base_time += orbit.periapsis * orbit.period_time / TAU;
        orbit.periapsis = 0.0;
    }
    // Make sure base_time is always between 0 and period_time
    orbit.base_time = positive_mod(orbit.base_time, orbit.period_time);
    orbit.inclination = positive_mod(orbit.inclination, TAU);
    orbit.ascending_node = positive_mod(orbit.ascending_node, TAU);
    orbit.periapsis = positive_mod(orbit.periapsis, TAU);
    orbit
}

fn orbits_eq(a: &OrbitData, b: &OrbitData) -> bool {
    let a = canonicalize_orbit(a);
    let b = canonicalize_orbit(b);
    // TODO: use better floating point comparison
    return a.semi_major == b.semi_major
        && a.semi_minor == b.semi_minor
        && a.inclination == b.inclination
        && a.ascending_node == b.ascending_node
        && a.periapsis == b.periapsis
        && a.base_time == b.base_time
        && a.period_time == b.period_time
        && a.parent == b.parent;
}

pub fn run_orbit_test(
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
    let orbit_value = state.get_property(ConnectionKey::null(), body, "orbit");
    match orbit_value {
        Ok(value) => {
            if value.is_null() {
                panic!("orbit is null instead of {:#?}", orbit);
            }
            match RequestResult::<OrbitData>::from(value) {
                Ok(actual) => {
                    if !orbits_eq(&actual, &orbit) {
                        panic!("expected {:#?}, got {:#?}", orbit, actual);
                    }
                },
                Err(e) => panic!("Orbit property produced invalid type: {}", e),
            }
        }
        Err(err) => panic!("getting orbit value produced error: {}", err),
    }
}
