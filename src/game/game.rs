use super::*;

struct CelestialInfo<'a> {
    name: &'a str,
    color: u32,
    parent: Id<Body>,
    distance: f64,
    mass: f64,
    radius: f64,
}

fn create_celestial(state: &mut State, scale: f64, info: CelestialInfo) -> Id<Body> {
    let (parent_pos, parent_vel, parent_mass) = state
        .get(info.parent)
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
        .with_velocity(Vector3::new(0.0, vel, 0.0) + parent_vel)
        .with_shape(Shape::from_radius(info.radius * scale).unwrap())
        .with_mass(info.mass * scale)
        .with_color(ColorRGB::from_u32(info.color))
        .with_name(info.name.to_string())
        .install(state)
}

// TODO: generalize create_celestial() to support non-circular, non-level orbits
fn create_planet_9(state: &mut State, scale: f64) {
    Body::new()
        .with_class(BodyClass::Celestial)
        .with_position(Point3::new(3.0e8, 0.0, 6.0e7) * scale)
        .with_velocity(Vector3::new(0.0, -12.0, 0.0))
        .with_shape(Shape::from_radius(12000.0 * scale).unwrap())
        .with_mass(6e+22 * scale)
        .with_color(ColorRGB::from_u32(0x2e5747))
        .with_name("Planet 9".to_string())
        .install(state);
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
            parent: Id::null(),
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

    create_planet_9(state, scale);
}

pub fn init(state: &mut State, config: &GameConfig) {
    init_solar_system(state, 0.000001);
    state.root.quit_at.set(config.max_game_time)
}

pub fn tick(state: &mut State) -> bool {
    let physics_dt = *state.root.physics_tick_duration;
    let min_roundtrip_time = *state.root.min_roundtrip_time;
    let time_per_time = *state.root.time_per_time;
    let physics_ticks =
        ((*state.root.network_tick_interval * time_per_time / physics_dt).ceil() as u64).min(5000);
    let effective_target_network_tick = if time_per_time > 0.0 {
        (physics_ticks as f64) * physics_dt / time_per_time
    } else {
        *state.root.network_tick_interval
    };
    state
        .metronome
        .set_params(effective_target_network_tick, min_roundtrip_time);

    for _ in 0..physics_ticks {
        *state.root.time.get_mut() += physics_dt;
        if let Some(pause_at) = *state.root.pause_at {
            if *state.root.time >= pause_at {
                state.root.time_per_time_will_be_set_to(0.0);
                state.root.time_per_time.set(0.0);
                state.root.pause_at.set(None);
                break;
            }
        }
        (physics_tick)(state, physics_dt);
        if check_pause_conditions(state) {
            state.root.time_per_time_will_be_set_to(0.0);
            state.root.time_per_time.set(0.0);
        }
    }

    if let Some(quit_at) = *state.root.quit_at {
        if *state.root.time > quit_at {
            info!(
                "engine has run for {:?}, stopping…",
                Duration::from_secs_f64(quit_at)
            );
            return true;
        }
    }
    false
}
