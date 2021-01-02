use super::*;

struct PendingUpdates {
    kill: bool,
}

impl PendingUpdates {
    fn new() -> Self {
        Self {
            kill: false,
        }
    }
}

pub struct Ship {
    max_thrust: f64,
    pub thrust: Element<Vector3<f64>>,
    #[allow(dead_code)]
    alive: bool,
    pending: Mutex<PendingUpdates>,
}

impl Ship {
    fn new(max_thrust: f64) -> Self {
        Self {
            max_thrust,
            thrust: Element::new(Vector3::zero()),
            alive: true,
            pending: Mutex::new(PendingUpdates::new()),
        }
    }

    fn set_thrust(&mut self, thrust: Vector3<f64>) -> Result<(), String> {
        let magnitude = thrust.magnitude();
        if magnitude > self.max_thrust + EPSILON {
            let fixed = thrust.normalize() * self.max_thrust;
            self.thrust.set(fixed);
            Err(format!(
                "{:?} has a magnitude of {}, which is greater than the maximum allowed thrust {}",
                thrust, magnitude, self.max_thrust
            ))
        } else {
            self.thrust.set(thrust);
            Ok(())
        }
    }

    fn kill(&self) {
        let mut pending = self
            .pending
            .lock()
            .expect("failed lock pending ship updates");
        pending.kill = true;
    }
}

struct ShipBodyController {
    ship: EntityKey,
}

impl CollisionHandler for ShipBodyController {
    fn collision(&self, state: &State, _collision: &Collision) {
        if let Ok(ship) = state.component::<Ship>(self.ship) {
            ship.kill();
        } else {
            error!("colliding ship {:?} does not exist", self.ship);
        }
    }
}

pub fn create_ship(state: &mut State, position: Point3<f64>, velocity: Vector3<f64>) -> EntityKey {
    let entity = state.create_entity();
    state.install_component(entity, Ship::new(1000.0));
    Body::new()
        .with_class(BodyClass::Ship)
        .with_position(position)
        .with_velocity(velocity)
        .with_sphere_shape(1.0)
        .with_collision_handler(Box::new(ShipBodyController { ship: entity }))
        .install(state, entity);
    RWConduit::new(
        move |state| Ok(&state.component::<Ship>(entity)?.thrust),
        move |state, value| state.component_mut::<Ship>(entity)?.set_thrust(value),
    )
    .install_property(state, entity, "thrust");
    info!("ship {:?} created at {:?}", entity, position);
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
