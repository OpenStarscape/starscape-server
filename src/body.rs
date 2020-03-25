use std::sync::Arc;
use cgmath::{Point3, Vector3};

use super::state::{State, BodyKey, GravityWellKey};

/// Type of body
#[derive(Clone, Copy)]
pub enum Type {
    Planetoid,
    Ship,
}

/// Collision shape
#[derive(Clone, Copy)]
pub enum Shape {
    Sphere { radius: f64 },
}

/// Any physics object in space
pub struct Body {
    pub position: Point3<f64>,
    pub velocity: Vector3<f64>,
    pub shape: Shape,
    pub type_: Type,
    pub gravity_well: Option<GravityWellKey>,
    pub brain: Option<Arc<dyn Brain>>,
}

impl Body {
    pub fn new(type_: Type, position: Point3<f64>, shape: Shape, brain: Option<Arc<dyn Brain>>) -> Body {
        Body {
            type_,
            position,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            shape,
            gravity_well: None,
            brain,
        }
    }
}

/// A source of gravity such as a planet or star
/// There is a performece impact for every gravity well, so objects with negligible mass (such as
/// ships and small asteroids) should not be gravity wells
pub struct GravityWell {
    pub pull: f64,
    pub body: BodyKey,
}

pub struct Collision {
    /// The time the collision will occur
    pub timestamp: f64,
    pub body: BodyKey,
}

pub trait Brain {
    fn collided_with(&self, state: &State, collision: &Collision);
}
