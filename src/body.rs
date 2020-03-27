use cgmath::{Point3, Vector3};
use std::sync::Arc;

use super::state::{BodyKey, State};

/// Collision shape
#[derive(Clone, Copy)]
pub enum Shape {
    Point,
    Sphere { radius: f64 },
}

/// Any physics object in space
pub struct Body {
    /// Location of the object (kilometers)
    /// (0, 0, 0) is generally the center of the solar system
    /// +Z is considered "up" from the orbital plane
    pub position: Point3<f64>,
    /// Speed at which the object is moving (kilometers-per-second)
    pub velocity: Vector3<f64>,
    /// Shape of this object (used for collision detection)
    pub shape: Shape,
    /// Mass of this object (kilotonnes aka millions of kilograms)
    pub mass: f64,
    /// If this object should be a source of gravity
    /// Ideally all objects would have a gravitational effect on all other objects, but that is
    /// unnecessary and computationally expensive.
    pub gravity_well: bool,
    /// The interface the physics system uses to talk to the controller of this object
    pub brain: Option<Arc<dyn Brain>>,
}

impl Body {
    pub fn new() -> Body {
        Body {
            position: Point3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            shape: Shape::Point,
            mass: 1.0,
            gravity_well: false,
            brain: None,
        }
    }

    pub fn with_position(mut self, position: Point3<f64>) -> Body {
        self.position = position;
        self
    }

    pub fn with_velocity(mut self, velocity: Vector3<f64>) -> Body {
        self.velocity = velocity;
        self
    }

    pub fn with_sphere_shape(mut self, radius: f64) -> Body {
        self.shape = Shape::Sphere { radius };
        self
    }

    pub fn with_mass(mut self, mass: f64) -> Body {
        self.mass = mass;
        self
    }

    pub fn with_gravity(mut self) -> Body {
        self.gravity_well = true;
        self
    }

    pub fn with_brain(mut self, brain: Arc<dyn Brain>) -> Body {
        self.brain = Some(brain);
        self
    }
}

pub struct Collision {
    /// The time the collision will occur
    pub timestamp: f64,
    pub body: BodyKey,
}

pub trait Brain {
    fn collided_with(&self, state: &State, collision: &Collision);
}
