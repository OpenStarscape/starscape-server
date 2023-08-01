use super::*;

fn env_var_parse_f64_or(var: &str, default: f64) -> f64 {
    return match std::env::var(var) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    };
}

lazy_static::lazy_static! {
    static ref KP: f64 = env_var_parse_f64_or("PID_P", 62.0);
    static ref KI: f64 = env_var_parse_f64_or("PID_I", -0.3);
    static ref KD: f64 = env_var_parse_f64_or("PID_D", 4.1);
    static ref MAX_I: f64 = env_var_parse_f64_or("PID_MAX_I", 0.02);
}

fn pid_autopilot(
    state: &mut State,
    dt: f64,
    ship_id: Id<Body>,
) -> Result<Vector3<f64>, Box<dyn Error>> {
    let ship = state.get(ship_id)?;
    let ship_pos = *ship.position;
    let ship_vel = *ship.velocity;
    let max_accel = *ship.ship()?.max_acceleration;
    let target_id = *ship.ship()?.autopilot.target;
    let target = state.get(target_id)?;
    let mut target_pos = *target.position;
    let mut target_vel = *target.velocity;
    let distance = ship
        .ship()?
        .autopilot
        .distance
        .unwrap_or(target.shape.radius() * 20.0);
    if distance > 0.0 {
        let gm = GRAVITATIONAL_CONSTANT * *target.mass;
        let period_time = TAU * (distance * distance * distance / gm).sqrt();
        let theta = ((*state.root.time / period_time) % 1.0) * TAU;
        target_pos += Vector3::new(f64::cos(theta), f64::sin(theta), 0.0) * distance;
        let orbit_speed = distance * TAU / period_time;
        target_vel += Vector3::new(-f64::sin(theta), f64::cos(theta), 0.0) * orbit_speed;
    }
    let error_vec = (target_pos - ship_pos) / (max_accel * max_accel);
    let error_vel = (target_vel - ship_vel) / max_accel;
    let autopilot = &mut state.get_mut(ship_id)?.ship_mut()?.autopilot;
    let p_value = *KP * error_vec;
    let i_value = *KI * autopilot.pid_accum;
    let d_value = *KD * error_vel;
    autopilot.pid_accum += error_vec * dt;
    let accum_len = autopilot.pid_accum.magnitude();
    if accum_len > *MAX_I {
        autopilot.pid_accum = (autopilot.pid_accum / accum_len) * *MAX_I;
    }
    Ok((p_value + i_value + d_value) * max_accel)
}

fn orbit(state: &mut State, dt: f64, ship_id: Id<Body>) -> Result<(), Box<dyn Error>> {
    //let params = orbit_params(state, ship_id)?;
    //let acceleration = accel_for_orbit(&params);
    let mut acceleration = pid_autopilot(state, dt, ship_id)?;
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
            AutopilotScheme::Orbit => orbit(state, dt, id),
        } {
            let ship = state.get_mut(id).unwrap().ship_mut().unwrap();
            ship.acceleration.set(Vector3::zero());
            ship.autopilot.scheme.set(AutopilotScheme::Off);
            error!("{:?} failed for {:?}: {}", scheme, id, err);
        }
    }
}
