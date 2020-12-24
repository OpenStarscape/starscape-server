//! All game logic belongs in this module, including ships, physics, etc

use super::*;

mod components;
#[allow(clippy::module_inception)]
mod game;
mod physics;

pub use game::{init, physics_tick};

use components::*;
use physics::*;

/// A very small value; used for floating-point comparisons
const EPSILON: f64 = 0.000_001;
