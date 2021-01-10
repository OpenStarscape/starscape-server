use super::*;

struct CelestialInfo<'a> {
    name: &'a str,
    color: u32,
    parent: EntityKey,
    distance: f64,
    mass: f64,
    radius: f64,
}

fn create_celestial(state: &mut State, scale: f64, info: CelestialInfo) -> EntityKey {
    let e = state.create_entity();
    let (parent_pos, parent_vel, parent_mass) = state
        .component::<Body>(info.parent)
        .map(|parent| (*parent.position, *parent.velocity, *parent.mass))
        .unwrap_or_else(|_| (Point3::origin(), Vector3::zero(), 0.0));
    let pos = parent_pos + Vector3::new(info.distance, 0.0, 0.0) * scale;
    let vel = if info.distance > EPSILON && parent_mass > EPSILON {
        let unscaled_parent_mass = parent_mass / scale;
        (GRAVITATIONAL_CONSTANT * unscaled_parent_mass / info.distance).sqrt() // for circular orbit
    } else {
        0.0
    };
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(pos)
        .with_velocity(Vector3::new(0.0, 0.0, vel) + parent_vel)
        .with_sphere_shape(info.radius * scale)
        .with_mass(info.mass * scale)
        .with_color(ColorRGB::from_u32(info.color))
        .with_name(info.name.to_string())
        .install(state, e);
    e
}

fn init_solar_system(state: &mut State, scale: f64) {
    // Note that scale affects mass, size and position but not velocity. This keeps orbits correct.

    // All values are intended to be correct for Sol (the Sun)
    let sol = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Sol",
            color: 0xffe461,
            parent: EntityKey::null(),
            distance: 0.0,
            mass: 1.989e+27,
            radius: 696340.0,
        },
    );

    // All values are intended to be correct for Mercury
    let _venus = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Mercury",
            color: 0xb89984,
            parent: sol,
            distance: 5.7389e+7,
            mass: 3.285e+20,
            radius: 2439.7,
        },
    );

    // All values are intended to be correct for Venus
    let _venus = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Venus",
            color: 0xbaa87d,
            parent: sol,
            distance: 1.0852e+8,
            mass: 4.867e+21,
            radius: 6051.8,
        },
    );

    // All values are intended to be correct for Earth
    let earth = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Earth",
            color: 0x1d55f0,
            parent: sol,
            distance: 1.496e+8,
            mass: 5.972e+21,
            radius: 6371.0,
        },
    );

    // All values are intended to be correct for Luna (Earth's moon)
    let _luna = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Luna",
            color: 0xd2d2d2,
            parent: earth,
            distance: 3.844e+5,
            mass: 7.34767309e+19,
            radius: 1737.0,
        },
    );

    // All values are intended to be correct for Mars
    let _mars = create_celestial(
        state,
        scale,
        CelestialInfo {
            name: "Mars",
            color: 0xd65733,
            parent: sol,
            distance: 2.2901e+8,
            mass: 6.39e+20,
            radius: 3389.5,
        },
    );
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
