use cgmath::*;
use slotmap::Key;

use crate::plumbing::Store;
use crate::state::{BodyKey, EntityKey, State};

/// Collision shape
#[derive(Clone, Copy, PartialEq)]
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
    pub entity: EntityKey,
    /// Location of the object (kilometers)
    /// (0, 0, 0) is generally the center of the solar system
    /// +Z is considered "up" from the orbital plane
    pub position: Store<Point3<f64>>,
    /// Speed at which the object is moving (kilometers-per-second)
    pub velocity: Store<Vector3<f64>>,
    /// Shape of this object (used for collision detection)
    pub shape: Store<Shape>,
    /// Mass of this object (kilotonnes aka millions of kilograms)
    pub mass: Store<f64>,
    /// If this object should be a source of gravity
    /// Ideally all objects would have a gravitational effect on all other objects, but that is
    /// unnecessary and computationally expensive.
    pub gravity_well: Store<bool>,
    /// The interface the physics system uses to talk to the controller of this object
    pub controller: Box<dyn Controller>,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            entity: EntityKey::null(),
            position: Store::new(Point3::origin()),
            velocity: Store::new(Vector3::zero()),
            shape: Store::new(Shape::Point),
            mass: Store::new(1.0),
            gravity_well: Store::new(false),
            controller: Box::new(()),
        }
    }
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entity(mut self, entity: EntityKey) -> Self {
        self.entity = entity;
        self
    }

    pub fn with_position(mut self, position: Point3<f64>) -> Self {
        self.position = Store::new(position);
        self
    }

    pub fn with_velocity(mut self, velocity: Vector3<f64>) -> Self {
        self.velocity = Store::new(velocity);
        self
    }

    pub fn with_sphere_shape(mut self, radius: f64) -> Self {
        self.shape = Store::new(Shape::Sphere { radius });
        self
    }

    pub fn with_mass(mut self, mass: f64) -> Self {
        self.mass = Store::new(mass);
        self
    }

    pub fn with_gravity(mut self) -> Self {
        self.gravity_well = Store::new(true);
        self
    }

    pub fn with_controller(mut self, controller: Box<dyn Controller>) -> Self {
        self.controller = controller;
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Collision {
    /// The time from now until the collision will occur
    pub time_until: f64,
    pub body: BodyKey,
}

impl Collision {
    pub fn new(time_until: f64, body: BodyKey) -> Collision {
        Collision { time_until, body }
    }
}

pub trait Controller {
    /// note that there is no guarantee collisions come in in order
    fn collided_with(&self, state: &State, collision: &Collision);
}

impl Controller for () {
    fn collided_with(&self, _state: &State, _collision: &Collision) {}
}
