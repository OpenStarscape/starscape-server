use super::*;

fn env_var_parse_f64_or(var: &str, default: f64) -> f64 {
    return match std::env::var(var) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    };
}

lazy_static::lazy_static! {
    static ref ACCEL_P: f64 = env_var_parse_f64_or("AP_ACCEL_P", 20.0);
    static ref DECEL_P: f64 = env_var_parse_f64_or("AP_DECEL_P", 1.0);
    static ref ALIGN_P: f64 = env_var_parse_f64_or("AP_ALIGN_P", 70.0);
}

fn normalize_or_zero(v: Vector3<f64>) -> Vector3<f64> {
    // Sometimes the vector comes in finite, but is not finite after normalization
    let result = v.normalize();
    if result.is_finite() {
        result
    } else {
        Vector3::zero()
    }
}

fn sign_of(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value <= 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn match_point(
    rel_target_pos: Vector3<f64>,
    rel_target_vel: Vector3<f64>,
    rel_target_accel: Vector3<f64>,
    max_accel: f64,
) -> Vector3<f64> {
    let target_direction = normalize_or_zero(rel_target_pos);
    // the magnitude of the component of the ship's velocity vector that is pointed towards the
    // target, can be negative
    let speed_moving_apart = rel_target_vel.dot(target_direction);
    // the acceleration of the target away from the ship
    let target_accel_away = rel_target_accel.dot(target_direction);
    // the acceleration of the target that is not aligned from the ship
    let target_accel_off_course = rel_target_accel - target_accel_away * target_direction;
    // the vector component of the ship's velocity that is pointed towards the target
    let target_component_of_ship_vel = -target_direction * speed_moving_apart;
    // the vector component of the ship's velocity that is *not* pointed twards the target
    let ship_vel_off_course = -rel_target_vel - target_component_of_ship_vel;
    // the max relative acceleration we can perform in the direction that reduces relative velocity. if this is ever
    // zero or negative we get weird results (accelerating away from target to meet it in the past) so don't let it go
    // below 1/10th max accel.
    let max_effective_rel_decel =
        (max_accel - sign_of(speed_moving_apart) * target_accel_away).max(max_accel / 10.0);
    // the distance before (or after if negative) the target which the ship would be at when it
    // stopped getting closer or further from the target if it put full thrust towards this
    // (assuming the ship is pointed directly at the target)
    let distance_at_vel_parity = rel_target_pos.magnitude()
        + speed_moving_apart * speed_moving_apart.abs() / (2.0 * max_effective_rel_decel);
    // the acceleration relative to the target's acceleration that would be required to match the target's position and
    // velocity (assuming the ship is pointed directly at the target)
    let rel_accel_to_match = speed_moving_apart * speed_moving_apart.abs()
        / (2.0 * rel_target_pos.magnitude().max(EPSILON));
    // the actual acceleration to match
    let accel_to_match = rel_accel_to_match + target_accel_away;
    let accel_vec = *ACCEL_P * distance_at_vel_parity * target_direction;
    let decel_vec = *DECEL_P * accel_to_match * target_direction;
    let align_vec = *ALIGN_P * -ship_vel_off_course;
    let result = accel_vec + decel_vec + align_vec + target_accel_off_course;
    result
}

fn flyby_point(
    rel_target_pos: Vector3<f64>,
    rel_target_vel: Vector3<f64>,
    target_accel: Vector3<f64>,
    max_accel: f64,
) -> Vector3<f64> {
    let target_direction = normalize_or_zero(rel_target_pos);
    // the magnitude of the component of the ship's velocity vector that is pointed towards the
    // target, can be negative
    let speed_moving_apart = rel_target_vel.dot(target_direction);
    // the vector component of the ship's velocity that is pointed towards the target
    let target_component_of_ship_vel = -target_direction * speed_moving_apart;
    // the vector component of the ship's velocity that is *not* pointed twards the target
    let ship_vel_off_course = -rel_target_vel - target_component_of_ship_vel;
    let accel_vec = *ACCEL_P * target_direction * max_accel;
    let align_vec = *ALIGN_P * -ship_vel_off_course;
    accel_vec + align_vec + target_accel
}

fn set_accel(
    state: &mut State,
    ship_id: Id<Body>,
    mut acceleration: Vector3<f64>,
) -> Result<Vector3<f64>, Box<dyn Error>> {
    let max_accel = *state.get(ship_id)?.ship()?.max_acceleration;
    if acceleration.magnitude2() > max_accel * max_accel {
        acceleration = acceleration.normalize() * max_accel;
    }
    if !acceleration.is_finite() {
        return Err(format!("acceleration {:?} is not finite", acceleration).into());
    }
    state
        .get_mut(ship_id)?
        .ship_mut()?
        .acceleration
        .set(acceleration);
    Ok(acceleration)
}

fn run_orbit(state: &mut State, dt: f64, ship_id: Id<Body>) -> Result<(), Box<dyn Error>> {
    let ship = state.get(ship_id)?;
    let self_pos = *ship.position;
    let self_vel = *ship.velocity;
    let max_accel = *ship.ship()?.max_acceleration;
    if max_accel <= 0.0 {
        return Err(format!("max_accel is {}", max_accel).into());
    }
    let autopilot_data = &ship.ship()?.autopilot;
    let target_id = *autopilot_data.target;
    let target = state.get(target_id)?;
    let mut target_pos = *target.position;
    let mut target_vel = *target.velocity;
    let emperical_rel_target_accel = if autopilot_data.previous.target_id == target_id
        && !matches!(target.class, BodyClass::Celestial)
    {
        let non_thrust_self_accel = (self_vel - autopilot_data.previous.self_vel) / dt
            - autopilot_data.previous.self_thrust;
        (target_vel - autopilot_data.previous.target_vel) / dt - non_thrust_self_accel
    } else {
        Vector3::zero()
    };
    let orbit_distance = if *autopilot_data.scheme == AutopilotScheme::Dock {
        0.0
    } else {
        autopilot_data
            .distance
            .unwrap_or(target.shape.radius() * 20.0)
    };
    if orbit_distance > 0.0 {
        // the current algorithm requires a point to navigate to, so if an orbit is requested
        // calculate a moving point on that orbit
        let gm = GRAVITATIONAL_CONSTANT * *target.mass;
        // period time of the target point
        let mut period_time = TAU * (orbit_distance.powi(3) / gm).sqrt();
        if !period_time.is_finite() || period_time <= 0.0 {
            // if the body doesn't have gravity, just spin a target point around it at an arbitrary
            // speed
            period_time = ((orbit_distance * 20.0) / max_accel).sqrt();
        }
        // the current angle of the the target point
        let theta = ((*state.root.time / period_time) % 1.0) * TAU;
        let quat = Quaternion::from_arc(
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, f64::sin(0.2), f64::cos(0.2)),
            None,
        );
        target_pos += (quat * Vector3::new(f64::cos(theta), f64::sin(theta), 0.0)) * orbit_distance;
        let orbit_speed = orbit_distance * TAU / period_time;
        target_vel += (quat * Vector3::new(-f64::sin(theta), f64::cos(theta), 0.0)) * orbit_speed;
    }
    let acceleration = match_point(
        target_pos - self_pos,
        target_vel - self_vel,
        emperical_rel_target_accel,
        max_accel,
    );
    // e/acc lol
    let effective_accel = set_accel(state, ship_id, acceleration)?;
    state.get_mut(ship_id)?.ship_mut()?.autopilot.previous = AutopilotDataPrevious {
        target_id,
        target_vel,
        self_vel,
        self_thrust: effective_accel,
    };
    Ok(())
}

fn run_flyby(state: &mut State, dt: f64, ship_id: Id<Body>) -> Result<(), Box<dyn Error>> {
    let ship = state.get(ship_id)?;
    let self_pos = *ship.position;
    let self_vel = *ship.velocity;
    let max_accel = *ship.ship()?.max_acceleration;
    if max_accel <= 0.0 {
        return Err(format!("max_accel is {}", max_accel).into());
    }
    let autopilot_data = &ship.ship()?.autopilot;
    let target_id = *autopilot_data.target;
    let target = state.get(target_id)?;
    let target_pos = *target.position;
    let target_vel = *target.velocity;
    let emperical_rel_target_accel = if autopilot_data.previous.target_id == target_id
        && !matches!(target.class, BodyClass::Celestial)
    {
        let non_thrust_self_accel = (self_vel - autopilot_data.previous.self_vel) / dt
            - autopilot_data.previous.self_thrust;
        (target_vel - autopilot_data.previous.target_vel) / dt - non_thrust_self_accel
    } else {
        Vector3::zero()
    };
    let acceleration = flyby_point(
        target_pos - self_pos,
        self_vel - target_vel,
        emperical_rel_target_accel,
        max_accel,
    );
    // e/acc lol
    let effective_accel = set_accel(state, ship_id, acceleration)?;
    state.get_mut(ship_id)?.ship_mut()?.autopilot.previous = AutopilotDataPrevious {
        target_id,
        target_vel,
        self_vel,
        self_thrust: effective_accel,
    };
    Ok(())
}

pub fn run_autopilot(state: &mut State, dt: f64) {
    // TODO: improve the ECS to make it easier to iterate through all ships
    let body_ids: Vec<Id<Body>> = state
        .iter::<Body>()
        .filter_map(|(id, body)| match body.class {
            BodyClass::Ship(_) => Some(id),
            _ => None,
        })
        .collect();
    for id in body_ids {
        let scheme = *state.get(id).unwrap().ship().unwrap().autopilot.scheme;
        if let Err(err) = match scheme {
            AutopilotScheme::Off => Ok(()),
            AutopilotScheme::Orbit => run_orbit(state, dt, id),
            AutopilotScheme::Dock => run_orbit(state, dt, id),
            AutopilotScheme::Flyby => run_flyby(state, dt, id),
        } {
            let ship = state.get_mut(id).unwrap().ship_mut().unwrap();
            ship.acceleration.set(Vector3::zero());
            ship.autopilot.scheme.set(AutopilotScheme::Off);
            error!("{:?} failed for {:?}: {}", scheme, id, err);
        }
    }
}
