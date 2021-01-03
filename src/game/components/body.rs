use super::*;

/// The threshold for how massive a body has to be to get a gravity body
const GRAVITY_BODY_THRESH: f64 = 100_000.0;

/// The type of object
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BodyClass {
    /// Stars, planets, moons and asteroids
    Celestial,
    /// Ships that may have thrust and be controlled
    Ship,
}

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
    /// Type of body
    pub class: Element<BodyClass>,
    /// Location of the object (kilometers)
    /// (0, 0, 0) is generally the center of the solar system
    /// +Z is considered "up" from the orbital plane
    pub position: Element<Point3<f64>>,
    /// Speed at which the object is moving (kilometers-per-second)
    pub velocity: Element<Vector3<f64>>,
    /// Shape of this object (used for collision detection)
    pub shape: Element<Shape>,
    /// Mass of this object (metric tons aka tonnes aka mt aka 1000s of kgs)
    pub mass: Element<f64>,
    /// The suggested color of the body, for display purposes only
    pub color: Element<Option<ColorRGB>>,
    /// The interface the physics system uses to talk to the controller of this object
    pub collision_handler: Box<dyn CollisionHandler>,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            class: Element::new(BodyClass::Celestial),
            position: Element::new(Point3::origin()),
            velocity: Element::new(Vector3::zero()),
            shape: Element::new(Shape::Point),
            mass: Element::new(1.0),
            color: Element::new(None),
            collision_handler: Box::new(()),
        }
    }
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class: BodyClass) -> Self {
        self.class = Element::new(class);
        self
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

    pub fn with_color(mut self, color: ColorRGB) -> Self {
        self.color = Element::new(Some(color));
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

        ROConduit::new(move |state| Ok(&state.component::<Body>(entity)?.class))
            .map_output(|class| {
                Ok(match class {
                    BodyClass::Celestial => "celestial".to_string(),
                    BodyClass::Ship => "ship".to_string(),
                })
            })
            .install_property(state, entity, "class");

        RWConduit::new(
            move |state| Ok(&state.component::<Body>(entity)?.position),
            move |state, value| Ok(state.component_mut::<Body>(entity)?.position.set(value)),
        )
        .install_property(state, entity, "position");

        RWConduit::new(
            move |state| Ok(&state.component::<Body>(entity)?.velocity),
            move |state, value| Ok(state.component_mut::<Body>(entity)?.velocity.set(value)),
        )
        .install_property(state, entity, "velocity");

        RWConduit::new(
            move |state| Ok(&state.component::<Body>(entity)?.mass),
            move |state, value| Ok(state.component_mut::<Body>(entity)?.mass.set(value)),
        )
        .install_property(state, entity, "mass");

        RWConduit::new(
            move |state| Ok(&state.component::<Body>(entity)?.color),
            move |state, value| Ok(state.component_mut::<Body>(entity)?.color.set(value)),
        )
        .install_property(state, entity, "color");

        RWConduit::new(
            move |state| Ok(&state.component::<Body>(entity)?.shape),
            move |state, value| Ok(state.component_mut::<Body>(entity)?.shape.set(value)),
        )
        .map_output(|shape| Ok(shape.radius()))
        .map_input(|radius| {
            if radius == 0.0 {
                Ok(Shape::Point)
            } else if radius > 0.0 {
                Ok(Shape::Sphere { radius })
            } else {
                Err("size must be >= 0".to_string())
            }
        })
        .install_property(state, entity, "size");
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
