use super::*;

fn env_var_parse_f64_or(var: &str, default: f64) -> f64 {
    return match std::env::var(var) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    };
}

lazy_static::lazy_static! {
    static ref ACCEL_P: f64 = env_var_parse_f64_or("AP_ACCEL_P", 20.0);
    static ref DECEL_P: f64 = env_var_parse_f64_or("AP_DECEL_P", 0.9);
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

fn match_point(
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
    // the distance before (or after if negative) the target which the ship would be at when it
    // stopped getting closer or further from the target if it put full thrust towards this
    // (assuming the ship is pointed directly at the target)
    let distance_at_vel_parity = rel_target_pos.magnitude()
        + speed_moving_apart * speed_moving_apart.abs() / (2.0 * max_accel);
    // the acceleration that would be required to match the target's position and velocity
    // (assuming the ship is pointed directly at the target)
    let accel_to_match = speed_moving_apart * speed_moving_apart.abs()
        / (2.0 * rel_target_pos.magnitude().max(EPSILON));
    let accel_vec = *ACCEL_P * distance_at_vel_parity * target_direction;
    let decel_vec = *DECEL_P * accel_to_match * target_direction;
    let align_vec = *ALIGN_P * -ship_vel_off_course;
    accel_vec + decel_vec + align_vec + target_accel
}

fn set_accel(
    state: &mut State,
    ship_id: Id<Body>,
    mut acceleration: Vector3<f64>,
) -> Result<(), Box<dyn Error>> {
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
    Ok(())
}

fn run_orbit(state: &mut State, dt: f64, ship_id: Id<Body>) -> Result<(), Box<dyn Error>> {
    let ship = state.get(ship_id)?;
    let ship_pos = *ship.position;
    let ship_vel = *ship.velocity;
    let max_accel = *ship.ship()?.max_acceleration;
    if max_accel <= 0.0 {
        return Err(format!("max_accel is {}", max_accel).into());
    }
    let autopilot_data = &ship.ship()?.autopilot;
    let target_id = *autopilot_data.target;
    let target = state.get(target_id)?;
    let mut target_pos = *target.position;
    let mut target_vel = *target.velocity;
    let emperical_target_accel = if autopilot_data.previous_target_vel.0 == target_id {
        (target_vel - autopilot_data.previous_target_vel.1) / dt
    } else {
        Vector3::zero()
    };
    let orbit_distance = autopilot_data
        .distance
        .unwrap_or(target.shape.radius() * 20.0);
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
        target_pos += Vector3::new(f64::cos(theta), f64::sin(theta), 0.0) * orbit_distance;
        let orbit_speed = orbit_distance * TAU / period_time;
        target_vel += Vector3::new(-f64::sin(theta), f64::cos(theta), 0.0) * orbit_speed;
    }
    state
        .get_mut(ship_id)?
        .ship_mut()?
        .autopilot
        .previous_target_vel = (target_id, target_vel);
    let acceleration = match_point(
        target_pos - ship_pos,
        target_vel - ship_vel,
        emperical_target_accel,
        max_accel,
    );
    set_accel(state, ship_id, acceleration)?;
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
        } {
            let ship = state.get_mut(id).unwrap().ship_mut().unwrap();
            ship.acceleration.set(Vector3::zero());
            ship.autopilot.scheme.set(AutopilotScheme::Off);
            error!("{:?} failed for {:?}: {}", scheme, id, err);
        }
    }
}
