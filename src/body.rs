use cgmath::{Point3, Vector3};

use super::state::{BodyKey, State};

/// Collision shape for any object (such as a planet or a ship)
#[derive(Clone, Copy)]
pub enum Shape {
    Sphere { radius: f64 },
}

struct Collider {
    position: Point3<f64>,
    velocity: Vector3<f64>,
    shape: Shape,
    body_key: BodyKey,
}

pub struct Collision {
    /// The time the collision will occur
    timestamp: f64,
    collider: Collider,
}

/// An object in space
pub trait Body {
    /// Should not mutate underlying data
    fn position(&self) -> Point3<f64>;
    /// Should not mutate underlying data
    fn velocity(&self) -> Vector3<f64>;
    /// Should not mutate underlying data
    fn collision_shape(&self) -> Shape;
    /// Should not mutate underlying data
    fn gravity_well<'a>(&'a self) -> Option<&'a dyn GravityWell> {
        None
    }
    /// May mutate underlying data
    /// If there are no collisions, step() should first add velocity to position
    /// If there are collisions, step() may apply changes to position and velocity based on them
    /// Non-position state (such as velocity) can then be updated
    /// collisions: a timestamp sorted list of things this body will collide with during this step
    fn step(&self, state: &State, start_time: f64, delta_time: f64, collisions: &[Collision]);
}

/// A source of gravity such as a planet or star
/// There is a performece impact for every gravity well, so objects with negligible mass (such as
/// ships and small asteroids) should not be gravity wells
pub trait GravityWell {
    fn position(&self) -> Point3<f64>;
    fn pull(&self) -> f64;
}
