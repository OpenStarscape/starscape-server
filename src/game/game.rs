use super::*;

fn init_solar_system(state: &mut State, scale: f64) {
    // Note that scale affects mass, size and position but not velocity. This keeps orbits correct.

    // All values are intended to be correct for Sol (the Sun)
    let sol = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(Point3::origin())
        .with_sphere_shape(696340.0 * scale)
        .with_mass(1.989e+27 * scale)
        .with_color(ColorRGB::from_u32(0xFFE060))
        .with_name("Sol".to_string())
        .install(state, sol);

    // All values are intended to be correct for Earth
    let earth = state.create_entity();
    let earth_pos = Point3::new(1.496e+8, 0.0, 0.0) * scale;
    let earth_vel = Vector3::new(0.0, 0.0, 30.0);
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(earth_pos)
        .with_velocity(earth_vel)
        .with_sphere_shape(6371.0 * scale)
        .with_mass(5.972e+21 * scale)
        .with_color(ColorRGB::from_u32(0x6090FF))
        .with_name("Earth".to_string())
        .install(state, earth);

    // All values are intended to be correct for Luna (Earth's moon)
    let luna = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(earth_pos + Vector3::new(3.844e5, 0.0, 0.0) * scale)
        .with_velocity(earth_vel + Vector3::new(0.0, 0.0, 1.022))
        .with_sphere_shape(1737.0 * scale)
        .with_mass(7.34767309e19 * scale)
        .with_color(ColorRGB::from_u32(0xD0D0D0))
        .with_name("Luna".to_string())
        .install(state, luna);
}

pub fn init(state: &mut State) {
    God::default().install(state);

    init_solar_system(state, 0.000001);
}

pub fn physics_tick(state: &mut State, delta: f64) {
    apply_acceleration(state, delta);
    apply_gravity(state, delta);
    apply_collisions(state, delta);
    apply_motion(state, delta);
    run_autopilot(state, delta);
}
