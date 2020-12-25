use super::*;

pub fn init(state: &mut State) {
    God::default().install(state);

    let _ship_a = create_ship(
        state,
        Point3::new(100_000.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 5_000.0),
    );

    let _ship_b = create_ship(
        state,
        Point3::new(0.0, 0.0, 60_000.0),
        Vector3::new(10_000.0, 1_000.0, 4_000.0),
    );

    let planet = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(Point3::origin())
        .with_sphere_shape(6_000.0)
        .with_mass(1.0e+15)
        .install(state, planet);

    let moon = state.create_entity();
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(Point3::new(60_000.0, 0.0, 0.0))
        .with_velocity(Vector3::new(0.0, 0.0, 10_000.0))
        .with_sphere_shape(2_000.0)
        .with_mass(1.0e+14)
        .install(state, moon);
}

pub fn physics_tick(state: &mut State, delta: f64) {
    apply_gravity(state, delta);
    apply_collisions(&state, delta);
    apply_motion(state, delta);
}
