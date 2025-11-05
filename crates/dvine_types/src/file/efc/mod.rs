//! Sound effect file support for `dvine-rs` project.

/// Constants used in `.EFC` files
pub mod constants {
	/// Maximum number of effects supported in `.EFC` files
	pub const MAX_EFFECTS: usize = 256;
}

/// File structure for `.EFC` files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct File {
	/// Index table mapping effect IDs to file offsets
	index_table: [u32; constants::MAX_EFFECTS],
}
