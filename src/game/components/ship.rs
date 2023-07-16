use super::*;

fn validate_thrust(max: f64, thrust: Vector3<f64>) -> (Vector3<f64>, RequestResult<()>) {
    let magnitude = thrust.magnitude();
    if magnitude <= max + EPSILON {
        (thrust, Ok(()))
    } else {
        let fixed = thrust.normalize() * max;
        let err =
            BadRequest(format!(
            "{:?} has a magnitude of {:?}, which is greater than the maximum allowed thrust {:?}",
            Value::from(thrust), Value::from(magnitude), Value::from(max)
        ));
        (fixed, Err(err))
    }
}

/// The autopilot program to use
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AutopilotScheme {
    /// No autopilot, client should set accel manually
    Off,
    /// Orbit a body with the given parameters
    /// - target: if null then orbits the currently most influential gravity body, else orbits the
    ///   given body
    /// - distance: distance from the target to orbit
    Orbit,
}

/// The data required for server-side control of a ship
pub struct AutopilotData {
    /// What data to use and how it is interpreted is dependent on the scheme
    pub scheme: Element<AutopilotScheme>,
    pub target: Element<Id<Body>>,
    pub distance: Element<Option<f64>>,
}

/// A vehicle that can maneuver under its own thrust
pub struct Ship {
    pub max_acceleration: Element<f64>,
    pub acceleration: Element<Vector3<f64>>,
    pub autopilot: AutopilotData,
}

impl Ship {
    fn new(max_acceleration: f64) -> Self {
        Self {
            max_acceleration: Element::new(max_acceleration),
            acceleration: Element::new(Vector3::zero()),
            autopilot: AutopilotData {
                scheme: Element::new(AutopilotScheme::Off),
                target: Element::new(Id::null()),
                distance: Element::new(None),
            },
        }
    }
}

pub fn create_ship(state: &mut State, position: Point3<f64>, velocity: Vector3<f64>) -> Id<Body> {
    let id = Body::new()
        .with_class(BodyClass::Ship(Ship::new(1.0))) // 100G (too much)
        .with_position(position)
        .with_velocity(velocity)
        .with_shape(Shape::from_radius(0.00001).unwrap())
        .install(state);
    let obj = state.object_mut(id).unwrap();

    obj.add_property(
        "max_accel",
        RWConduit::new(
            move |state| Ok(&state.get(id)?.ship()?.max_acceleration),
            move |state| Ok(&mut state.get_mut(id)?.ship_mut()?.max_acceleration),
        )
        .map_input(move |state, max| {
            let accel = &mut state.get_mut(id)?.ship_mut()?.acceleration;
            let (fixed, _) = validate_thrust(max, **accel);
            accel.set(fixed);
            Ok((max, Ok(())))
        })
        .map_into(),
    );

    obj.add_property(
        "accel",
        RWConduit::new(
            move |state| Ok(&state.get(id)?.ship()?.acceleration),
            move |state| Ok(&mut state.get_mut(id)?.ship_mut()?.acceleration),
        )
        .map_input(move |state, accel| {
            Ok(validate_thrust(
                *state.get(id)?.ship()?.max_acceleration,
                accel,
            ))
        })
        .map_into(),
    );

    obj.add_property(
        "ap_scheme",
        RWConduit::new(
            move |state| Ok(&state.get(id)?.ship()?.autopilot.scheme),
            move |state| Ok(&mut state.get_mut(id)?.ship_mut()?.autopilot.scheme),
        )
        .map_output(|_, scheme| {
            Ok(match scheme {
                AutopilotScheme::Off => "off".to_string(),
                AutopilotScheme::Orbit => "orbit".to_string(),
            })
        })
        .map_input(|_, scheme: String| match &scheme[..] {
            "off" => Ok((AutopilotScheme::Off, Ok(()))),
            "orbit" => Ok((AutopilotScheme::Orbit, Ok(()))),
            _ => Err(BadRequest(format!(
                "{:?} is an invalid autopilot scheme",
                scheme
            ))),
        })
        .map_into(),
    );

    obj.add_property(
        "ap_target",
        RWConduit::new_into(
            move |state| Ok(&state.get(id)?.ship()?.autopilot.target),
            move |state| Ok(&mut state.get_mut(id)?.ship_mut()?.autopilot.target),
        ),
    );

    obj.add_property(
        "ap_distance",
        RWConduit::new_into(
            move |state| Ok(&state.get(id)?.ship()?.autopilot.distance),
            move |state| Ok(&mut state.get_mut(id)?.ship_mut()?.autopilot.distance),
        ),
    );

    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_correct_position() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let ship = create_ship(&mut state, pos, Vector3::zero());
        assert_eq!(*state.get(ship).unwrap().position, pos);
    }

    #[test]
    fn body_has_sphere_shape() {
        let mut state = State::new();
        let ship = create_ship(&mut state, Point3::new(1.0, 2.0, 3.0), Vector3::zero());
        assert_eq!(
            *state.get(ship).unwrap().shape,
            body::Shape::Sphere { radius: 0.00001 }
        );
    }
}
