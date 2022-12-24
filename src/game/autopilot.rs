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
    /// The distance from the gravity body we want to be orbiting at, can be infinity
    goal_altitude: f64,
    /// The up direction of the orbit axis, such that the orbit is counter-clockwise from the top
    goal_axis: Option<Vector3<f64>>,
    /// The direction from the grav body we want to be at (normalized)
    goal_virtical_direction: Option<Vector3<f64>>,
}

fn orbit_params(state: &State, ship_key: EntityKey) -> Result<OrbitParams, Box<dyn Error>> {
    let ship = state.component::<Ship>(ship_key)?;
    let body = state.component::<Body>(ship_key)?;
    let grav_body_key = *body.gravity_parent;
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
        .get(grav_body_key)
        .map_err(|e| format!("getting gravity body: {}", e))?;
    let grav_body_pos = *grav_body.position;
    let grav_body_vel = *grav_body.velocity;
    let grav_body_mass = *grav_body.mass;
    let goal_altitude;
    let goal_axis;
    let goal_virtical_direction;
    if grav_body_key == target_key {
        goal_altitude = match *ship.autopilot.distance {
            Some(d) => d,
            None => grav_body.shape.radius() * 4.0 + 0.5,
        };
        goal_axis = None;
        goal_virtical_direction = None;
    } else {
        let mut temp_target = state.get(target_key);
        while let Ok(attempted_temp_target) = temp_target {
            if *attempted_temp_target.gravity_parent == grav_body_key {
                break;
            }
            temp_target = state.get(*attempted_temp_target.gravity_parent);
        }
        if let Ok(temp_target) = temp_target {
            let target_relative_pos = *temp_target.position - grav_body_pos;
            goal_altitude = target_relative_pos.magnitude();
            goal_axis = Some(target_relative_pos.cross(*temp_target.velocity).normalize());
            goal_virtical_direction = Some(target_relative_pos.normalize());
        // TODO: take into account that the target's orbit may be elliptical
        } else {
            goal_altitude = f64::INFINITY;
            goal_axis = None;
            goal_virtical_direction = None;
        }
    }
    Ok(OrbitParams {
        position,
        velocity,
        max_acceleration,
        grav_body_pos,
        grav_body_vel,
        grav_body_mass,
        goal_altitude,
        goal_axis,
        goal_virtical_direction,
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
    let forward_velocity_error;
    let vertical_velocity_error;
    if params.goal_altitude.is_finite() {
        let forward_velocity = lateral_velocity.magnitude();
        let final_forward_velocity =
            (GRAVITATIONAL_CONSTANT * params.grav_body_mass / params.goal_altitude).sqrt();
        let goal_angular_momentum = final_forward_velocity * params.goal_altitude; // * our mass but that cancels out
        let goal_forward_velocity = goal_angular_momentum / altitude;
        forward_velocity_error = goal_forward_velocity - forward_velocity;
        let goal_altitude = match params.goal_virtical_direction {
            None => params.goal_altitude,
            Some(goal_virtical_direction) => {
                let delta_theta = goal_virtical_direction.dot(vertical_direction).acos();
                let delta_theta = if (goal_virtical_direction - vertical_direction)
                    .normalize()
                    .dot(lateral_direction)
                    > 0.0
                {
                    delta_theta
                } else {
                    -delta_theta
                };
                // delta_theta is now the angle (positive or negative) to the target place in the orbit
                let scale = 1.0 - delta_theta / TAU;
                params.goal_altitude * scale
            }
        };
        let goal_vertical_velocity = goal_altitude - altitude; // achieve goal in ~1s
        vertical_velocity_error = goal_vertical_velocity - vertical_velocity;
    } else {
        forward_velocity_error = params.max_acceleration;
        vertical_velocity_error = 0.0;
    }
    let pitch_error = match params.goal_axis {
        Some(axis) => vertical_direction.cross(-axis).normalize() - lateral_direction,
        None => Vector3::zero(),
    };
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
