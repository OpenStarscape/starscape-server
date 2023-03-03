use super::*;

/// The threshold for how massive a body has to be to get a gravity body
const GRAVITY_BODY_THRESH: f64 = 100_000.0;

/// The type of object
pub enum BodyClass {
    /// Stars, planets, moons and asteroids
    Celestial,
    /// Ships that may have thrust and be controlled
    Ship(Ship),
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

/// Any physics object in space
pub struct Body {
    /// Type of body
    pub class: BodyClass,
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
    /// Human-readable name (generally capitalized with spaces)
    pub name: Element<Option<String>>,
    /// The least massive body that is more massive than this body, and for which this body is within the sphere of
    /// influence of (https://en.wikipedia.org/wiki/Sphere_of_influence_(astrodynamics)). This logic generally results
    /// in a nice tree. For example, a ship's parent might be Luna, Luna's parent would be Earth and Earth's parent
    /// would be Sol.
    pub gravity_parent: Element<Id<Body>>,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            /// Must not be changed once body installed
            class: BodyClass::Celestial,
            position: Element::new(Point3::origin()),
            velocity: Element::new(Vector3::zero()),
            shape: Element::new(Shape::Point),
            mass: Element::new(1.0),
            color: Element::new(None),
            name: Element::new(None),
            gravity_parent: Element::new(Id::null()),
        }
    }
}

macro_rules! add_rw_property {
    ( $obj:ident, $name:ident, $id:ident, $($getter:tt)* ) => {
        {
            $obj.add_property(
                stringify!($name),
                RWConduit::new(
                    move |state| Ok(&state.get($id)?.$($getter)*),
                    move |state, value| Ok(state.get_mut($id)?.$($getter)*.set(value)),
                )
                .map_into::<Value, Value>(),
            );
        }
    };
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class: BodyClass) -> Self {
        self.class = class;
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

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Element::new(Some(name));
        self
    }

    /// Attaches the body to the given entty, and adds a gravity body if the mass is at least
    /// GRAVITY_BODY_THRESH
    pub fn install(self, state: &mut State) -> Id<Body> {
        let class_name = match self.class {
            BodyClass::Celestial => "celestial".to_string(),
            BodyClass::Ship(_) => "ship".to_string(),
        };

        let (id, obj) = state.add_with_object(self);

        obj.add_property(
            "class",
            ConstConduit::new(class_name).map_into::<Value, Value>(),
        );

        obj.add_property("orbit", OrbitConduit::new(id).map_into::<Value, Value>());

        add_rw_property!(obj, position, id, position);
        add_rw_property!(obj, velocity, id, velocity);
        add_rw_property!(obj, mass, id, mass);
        add_rw_property!(obj, color, id, color);
        add_rw_property!(obj, name, id, name);

        obj.add_property(
            "grav_parent",
            ROConduit::new(move |state| Ok(&state.get(id)?.gravity_parent))
                .map_into::<Value, Value>(),
        );

        obj.add_property(
            "size",
            RWConduit::new(
                move |state| Ok(&state.get(id)?.shape),
                move |state, value| Ok(state.get_mut(id)?.shape.set(value)),
            )
            .map_output(|shape| Ok(shape.radius()))
            .map_input(|radius| {
                if radius == 0.0 {
                    Ok(Shape::Point)
                } else if radius > 0.0 {
                    Ok(Shape::Sphere { radius })
                } else {
                    Err(BadRequest("size must be >= 0".into()))
                }
            })
            .map_into::<Value, Value>(),
        );
        id
    }

    pub fn is_gravity_well(&self) -> bool {
        *self.mass > GRAVITY_BODY_THRESH
    }

    pub fn ship(&self) -> RequestResult<&Ship> {
        match &self.class {
            BodyClass::Ship(ship) => Ok(ship),
            _ => Err(InternalError("body is not a ship".to_string())),
        }
    }

    pub fn ship_mut(&mut self) -> RequestResult<&mut Ship> {
        match &mut self.class {
            BodyClass::Ship(ship) => Ok(ship),
            _ => Err(InternalError("body is not a ship".to_string())),
        }
    }
}
