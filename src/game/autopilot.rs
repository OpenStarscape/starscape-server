use super::*;

/// Parameters to calculate acceleration required to achieve a specific orbit. The algorithm that
/// uses this assumes we're currently orbiting around the gravity body, and no other gravity wells
/// have a significant effect.
struct OrbitParams {
    /// Our current position
    position: Point3<f64>,
    /// Our current velocity
    velocity: Vector3<f64>,
    /// Our max acceleration
    max_acceleration: f64,
    /// Position of the body we are currently orbiting around
    grav_body_pos: Point3<f64>,
    /// Velocity of the body we are currently orbiting around
    grav_body_vel: Vector3<f64>,
    /// Mass of the body we are currently orbiting around
    grav_body_mass: f64,
    /// The distance from the gravity body we want to be orbiting at
    goal_altitude: f64,
    /// The up direction of the orbit axis, such that the orbit is counter-clockwise from the top
    goal_axis: Vector3<f64>,
}

fn orbit_params(state: &State, ship_key: EntityKey) -> Result<OrbitParams, Box<dyn Error>> {
    let ship = state.component::<Ship>(ship_key)?;
    let body = state.component::<Body>(ship_key)?;
    let grav_body_key = *body.most_influential_gravity_body;
    let target_key = if !ship.autopilot.target.is_null() {
        *ship.autopilot.target
    } else if !grav_body_key.is_null() {
        grav_body_key
    } else {
        return Err("no viable target".into());
    };
    let position = *body.position;
    let velocity = *body.velocity;
    let max_acceleration = *ship.max_acceleration;
    let grav_body = state
        .component::<Body>(grav_body_key)
        .map_err(|e| format!("getting gravity body: {}", e))?;
    let grav_body_pos = *grav_body.position;
    let grav_body_vel = *grav_body.velocity;
    let grav_body_mass = *grav_body.mass;
    let goal_altitude;
    if grav_body_key == target_key {
        goal_altitude = match *ship.autopilot.distance {
            Some(d) => d,
            None => grav_body.shape.radius() * 4.0 + 0.5,
        }
    } else {
        let target_body = state
            .component::<Body>(target_key)
            .map_err(|e| format!("getting target body: {}", e))?;
        let target_position = *target_body.position;
        goal_altitude = target_position.distance(grav_body_pos);
        // TODO: take into account that the target's orbit may be elliptical
        // TODO: somehow actually line up with the target
    }
    let goal_axis = Vector3::new(0.0, 1.0, 0.0);
    Ok(OrbitParams {
        position,
        velocity,
        max_acceleration,
        grav_body_pos,
        grav_body_vel,
        grav_body_mass,
        goal_altitude,
        goal_axis,
    })
}

fn accel_for_orbit(params: &OrbitParams) -> Vector3<f64> {
    let relative_pos = params.position - params.grav_body_pos;
    let relative_vel = params.velocity - params.grav_body_vel;
    let vertical_direction = relative_pos.normalize();
    let altitude = relative_pos.magnitude();
    let vertical_velocity = vertical_direction.dot(relative_vel);
    let lateral_velocity = relative_vel - (vertical_velocity * vertical_direction);
    let lateral_direction = lateral_velocity.normalize();
    let forward_velocity = lateral_velocity.magnitude();
    let final_forward_velocity =
        (GRAVITATIONAL_CONSTANT * params.grav_body_mass / params.goal_altitude).sqrt();
    let goal_angular_momentum = final_forward_velocity * params.goal_altitude; // * our mass but that cancels out
    let goal_forward_velocity = goal_angular_momentum / altitude;
    let forward_velocity_error = goal_forward_velocity - forward_velocity;
    let goal_vertical_velocity = params.goal_altitude - altitude; // achieve goal in ~1s
    let vertical_velocity_error = goal_vertical_velocity - vertical_velocity;
    let pitch_error = vertical_direction.cross(-params.goal_axis).normalize() - lateral_direction;
    let ideal_accel = lateral_direction * forward_velocity_error
        + vertical_direction * vertical_velocity_error
        + pitch_error * 10.0;
    if ideal_accel.magnitude() <= params.max_acceleration {
        ideal_accel
    } else {
        ideal_accel.normalize() * params.max_acceleration
    }
}

fn orbit(state: &mut State, ship_key: EntityKey) -> Result<(), Box<dyn Error>> {
    let params = orbit_params(state, ship_key)?;
    let acceleration = accel_for_orbit(&params);
    state
        .component_mut::<Ship>(ship_key)?
        .acceleration
        .set(acceleration);
    Ok(())
}

pub fn run_autopilot(state: &mut State, _: f64) {
    // TODO: improve the ECS so we don't need to collect a vec here
    let ships: Vec<EntityKey> = state.components_iter::<Ship>().map(|(e, _)| e).collect();
    for ship_key in ships {
        if let Ok(ship) = state.component::<Ship>(ship_key) {
            let scheme = *ship.autopilot.scheme;
            if let Err(err) = match scheme {
                AutopilotScheme::Off => Ok(()),
                AutopilotScheme::Orbit => orbit(state, ship_key),
            } {
                if let Ok(ship) = state.component_mut::<Ship>(ship_key) {
                    ship.acceleration.set(Vector3::zero());
                    ship.autopilot.scheme.set(AutopilotScheme::Off);
                }
                error!("{:?} failed for {:?}: {}", scheme, ship_key, err);
            }
        }
    }
}
