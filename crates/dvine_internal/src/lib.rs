//! This module is separated into its own crate to enable simple dynamic linking for `dvine`, and should not be used directly.

/// `use dvine::prelude::*;` to import commonly used items.
pub mod prelude;

// Re-export dvine_types for convenience
pub use dvine_types;

// Re-export commonly used types at crate root
pub use dvine_types::file::{BLOCK_SIZE, DskError, DskFile, Entry, PftError, PftFile, PftHeader};
