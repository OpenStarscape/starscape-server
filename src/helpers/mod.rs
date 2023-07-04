//! General useful bits and bobs

use super::*;

mod color_rgb;
mod config;
mod datagram_splitter;
mod filesystem;
mod format_slotmap_key;
mod initializable;
mod metronome;
mod or_log;
mod short_type_name;
#[cfg(test)]
mod test_helpers;
mod thin_ptr;

pub use self::config::*;
pub use color_rgb::ColorRGB;
pub use datagram_splitter::DatagramSplitter;
pub use filesystem::*;
pub use format_slotmap_key::format_slotmap_key;
pub use initializable::Initializable;
pub use metronome::Metronome;
pub use or_log::OrLog;
pub use short_type_name::short_type_name;
#[cfg(test)]
pub use test_helpers::*;
pub use thin_ptr::ThinPtr;

pub trait AssertIsSync: Sync {}
