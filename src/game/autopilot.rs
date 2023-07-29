use super::*;

const KP: f64 = 6.2;
const KI: f64 = -0.03;
const KD: f64 = 4.1;
const MAX_I: f64 = 2.0;

/*
lazy_static::lazy_static! {
    static ref KP: f64 = std::env::var("KP").unwrap().parse().unwrap();
    static ref KI: f64 = std::env::var("KI").unwrap().parse().unwrap();
    static ref KD: f64 = std::env::var("KD").unwrap().parse().unwrap();
}
*/

fn pid_autopilot(
    state: &mut State,
    dt: f64,
    ship_id: Id<Body>,
) -> Result<Vector3<f64>, Box<dyn Error>> {
    let ship = state.get(ship_id)?;
    let ship_pos = *ship.position;
    let ship_vel = *ship.velocity;
    let target_id = *state.get(ship_id)?.ship()?.autopilot.target;
    let target = state.get(target_id)?;
    let target_body_pos = *target.position;
    let target_body_vel = *target.velocity;
    let distance = ship
        .ship()?
        .autopilot
        .distance
        .unwrap_or(target.shape.radius() * 4.0);
    let target_pos = target_body_pos + (ship_pos - target_body_pos).normalize_to(distance);
    let error_vec = target_pos - ship_pos;
    let error_vel = target_body_vel - ship_vel;
    let autopilot = &mut state.get_mut(ship_id)?.ship_mut()?.autopilot;
    let p_value = KP * error_vec;
    let i_value = KI * autopilot.pid_accum;
    let d_value = KD * error_vel;
    autopilot.pid_accum += error_vec * dt;
    let accum_len = autopilot.pid_accum.magnitude();
    if accum_len > MAX_I {
        autopilot.pid_accum = (autopilot.pid_accum / accum_len) * MAX_I;
    }
    Ok(p_value + i_value + d_value)
}

fn orbit(state: &mut State, dt: f64, ship_id: Id<Body>) -> Result<(), Box<dyn Error>> {
    //let params = orbit_params(state, ship_id)?;
    //let acceleration = accel_for_orbit(&params);
    let mut acceleration = pid_autopilot(state, dt, ship_id)?;
    let mag = acceleration.magnitude();
    let max_accel = *state.get(ship_id)?.ship()?.max_acceleration;
    if mag > max_accel {
        acceleration = acceleration * (max_accel / mag);
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
