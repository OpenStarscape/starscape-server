use super::*;

/// The threshold for how massive a body has to be to get a gravity body
const GRAVITY_BODY_THRESH: f64 = 100_000.0;

/// Collision shape
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shape {
    Point,
    Sphere { radius: f64 },
}

impl Shape {
    pub fn radius(&self) -> f64 {
        match self {
            Shape::Point => 0.0,
            Shape::Sphere { radius } => *radius,
        }
    }
}

/// Empty type that indicates this entity is a source of gravity
/// Ideally all objects would have a gravitational effect on all other objects, but that is
/// unnecessary and computationally expensive
pub struct GravityBody;

/// Any physics object in space
pub struct Body {
    /// Location of the object (kilometers)
    /// (0, 0, 0) is generally the center of the solar system
    /// +Z is considered "up" from the orbital plane
    pub position: Element<Point3<f64>>,
    /// Speed at which the object is moving (kilometers-per-second)
    pub velocity: Element<Vector3<f64>>,
    /// Shape of this object (used for collision detection)
    pub shape: Element<Shape>,
    /// Mass of this object (kilotonnes aka millions of kilograms)
    pub mass: Element<f64>,
    /// The interface the physics system uses to talk to the controller of this object
    pub collision_handler: Box<dyn CollisionHandler>,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: Element::new(Point3::origin()),
            velocity: Element::new(Vector3::zero()),
            shape: Element::new(Shape::Point),
            mass: Element::new(1.0),
            collision_handler: Box::new(()),
        }
    }
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, position: Point3<f64>) -> Self {
        self.position = Element::new(position);
        self
    }

    #[allow(dead_code)]
    pub fn with_velocity(mut self, velocity: Vector3<f64>) -> Self {
        self.velocity = Element::new(velocity);
        self
    }

    pub fn with_sphere_shape(mut self, radius: f64) -> Self {
        self.shape = Element::new(Shape::Sphere { radius });
        self
    }

    pub fn with_mass(mut self, mass: f64) -> Self {
        self.mass = Element::new(mass);
        self
    }

    pub fn with_collision_handler(mut self, controller: Box<dyn CollisionHandler>) -> Self {
        self.collision_handler = controller;
        self
    }

    /// Attaches the body to the given entty, and adds a gravity body if the mass is at least
    /// GRAVITY_BODY_THRESH
    pub fn install(self, state: &mut State, entity: EntityKey) {
        if *self.mass >= GRAVITY_BODY_THRESH {
            state.install_component(entity, GravityBody);
        }
        state.install_component(entity, self);
        state.install_property(
            entity,
            "position",
            Box::new(ElementConduit::new(
                move |state: &State| Ok(&state.component::<Body>(entity)?.position),
                move |state: &mut State, value: &Decoded| {
                    let (notifs, body) = state.component_mut::<Body>(entity)?;
                    body.position.set(notifs, value.try_get()?);
                    Ok(())
                },
            )),
        );
        state.install_property(
            entity,
            "mass",
            Box::new(ElementConduit::new(
                move |state: &State| Ok(&state.component::<Body>(entity)?.mass),
                move |state: &mut State, value: &Decoded| {
                    let (notifs, body) = state.component_mut::<Body>(entity)?;
                    body.mass.set(notifs, value.try_get()?);
                    Ok(())
                },
            )),
        );
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Collision {
    /// The time from now until the collision will occur
    pub time_until: f64,
    pub body: EntityKey,
}

impl Collision {
    pub fn new(time_until: f64, body: EntityKey) -> Collision {
        Collision { time_until, body }
    }
}

pub trait CollisionHandler {
    /// note that there is no guarantee collisions come in in order
    fn collision(&self, state: &State, collision: &Collision);
}

impl CollisionHandler for () {
    fn collision(&self, _state: &State, _collision: &Collision) {}
}
