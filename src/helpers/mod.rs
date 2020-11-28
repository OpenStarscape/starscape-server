//! General useful bits and bobs

#[cfg(test)]
use super::*;

mod datagram_splitter;
#[cfg(test)]
mod test_helpers;
mod thin_ptr;

pub use datagram_splitter::DatagramSplitter;
#[cfg(test)]
pub use test_helpers::*;
pub use thin_ptr::ThinPtr;
