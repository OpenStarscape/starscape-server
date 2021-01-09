use super::*;

/// The autopilot program to use
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AutopilotScheme {
    /// No autopilot, client should set accel manually
    Off,
    /// Orbit the target body at the specified distance (or a reasonable default if None)
    Orbit,
}

/// The data required for server-side control of a ship
pub struct AutopilotData {
    /// What data to use and how it is interpreted is dependent on the scheme
    pub scheme: Element<AutopilotScheme>,
    pub target: Element<EntityKey>,
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
                target: Element::new(EntityKey::null()),
                distance: Element::new(None),
            },
        }
    }

    fn set_thrust(&mut self, thrust: Vector3<f64>) -> Result<(), String> {
        let magnitude = thrust.magnitude();
        if magnitude > *self.max_acceleration + EPSILON {
            let fixed = thrust.normalize() * *self.max_acceleration;
            self.acceleration.set(fixed);
            Err(format!(
                "{:?} has a magnitude of {}, which is greater than the maximum allowed thrust {}",
                thrust, magnitude, *self.max_acceleration
            ))
        } else {
            self.acceleration.set(thrust);
            Ok(())
        }
    }
}

struct ShipBodyController {
    ship: EntityKey,
}

impl CollisionHandler for ShipBodyController {
    fn collision(&self, state: &State, _collision: &Collision) {
        if let Ok(_ship) = state.component::<Ship>(self.ship) {
            // TODO: destroy ship?
        } else {
            error!("colliding ship {:?} does not exist", self.ship);
        }
    }
}

pub fn create_ship(state: &mut State, position: Point3<f64>, velocity: Vector3<f64>) -> EntityKey {
    let entity = state.create_entity();

    Body::new()
        .with_class(BodyClass::Ship)
        .with_position(position)
        .with_velocity(velocity)
        .with_sphere_shape(1.0)
        .with_collision_handler(Box::new(ShipBodyController { ship: entity }))
        .install(state, entity);

    state.install_component(entity, Ship::new(1.0)); // 100G (too much)

    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.max_acceleration),
        move |state, value| {
            Ok(state
                .component_mut::<Ship>(entity)?
                .max_acceleration
                .set(value))
        },
    )
    .install_property(state, entity, "max_accel");

    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.acceleration),
        move |state, value| state.component_mut::<Ship>(entity)?.set_thrust(value),
    )
    .install_property(state, entity, "accel");

    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.autopilot.scheme),
        move |state, value| {
            Ok(state
                .component_mut::<Ship>(entity)?
                .autopilot
                .scheme
                .set(value))
        },
    )
    .map_output(|scheme| {
        Ok(match scheme {
            AutopilotScheme::Off => "off".to_string(),
            AutopilotScheme::Orbit => "orbit".to_string(),
        })
    })
    .map_input(|scheme: String| match &scheme[..] {
        "off" => Ok(AutopilotScheme::Off),
        "orbit" => Ok(AutopilotScheme::Orbit),
        _ => Err(format!("{:?} is an invalid autopilot scheme", scheme)),
    })
    .install_property(state, entity, "ap_scheme");

    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.autopilot.target),
        move |state, value| {
            Ok(state
                .component_mut::<Ship>(entity)?
                .autopilot
                .target
                .set(value))
        },
    )
    .install_property(state, entity, "ap_target");

    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.autopilot.distance),
        move |state, value| {
            Ok(state
                .component_mut::<Ship>(entity)?
                .autopilot
                .distance
                .set(value))
        },
    )
    .install_property(state, entity, "ap_distance");

    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_correct_position() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let mut state = State::new();
        let ship = create_ship(&mut state, pos, Vector3::zero());
        assert_eq!(*state.component::<Body>(ship).unwrap().position, pos);
    }

    #[test]
    fn body_has_sphere_shape() {
        let mut state = State::new();
        let ship = create_ship(&mut state, Point3::new(1.0, 2.0, 3.0), Vector3::zero());
        assert_eq!(
            *state.component::<Body>(ship).unwrap().shape,
            body::Shape::Sphere { radius: 1.0 }
        );
    }
}
