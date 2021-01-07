use super::*;

pub struct Ship {
    pub max_acceleration: Element<f64>,
    pub acceleration: Element<Vector3<f64>>,
}

impl Ship {
    fn new(max_acceleration: f64) -> Self {
        Self {
            max_acceleration: Element::new(max_acceleration),
            acceleration: Element::new(Vector3::zero()),
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

    state.install_component(entity, Ship::new(0.1)); // about 10Gs

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
