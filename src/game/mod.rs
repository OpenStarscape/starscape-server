//! All game logic belongs in this module, including ships, physics, etc

use super::*;

mod components;
mod game;
mod systems;

pub use game::Game;

use components::*;
use systems::*;
