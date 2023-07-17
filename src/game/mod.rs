//! All game logic belongs in this module, including ships, physics, etc

use super::*;

mod autopilot;
mod components;
mod conduits;
mod game;
mod game_config;
mod physics;
#[cfg(test)]
mod test;

pub use components::*;
pub use game::{init, tick};
pub use game_config::*;

use autopilot::*;
use conduits::*;
use physics::*;

/// A very small value; used for floating-point comparisons
const EPSILON: f64 = 0.000_001;
