//! Prelude module for commonly used items.
//!
//! This module provides a convenient way to import commonly used types and traits.
//!
//! # Examples
//!
//! ```rust
//! use dvine_internal::prelude::*;
//!
//! // Now you can use DskFile, PftFile, Entry, etc. directly
//! let dsk = DskFile::new();
//! let pft = PftFile::empty();
//! ```

#[doc(inline)]
pub use crate::dvine_types::file::{
	BLOCK_SIZE, DskError, DskFile, Entry, PftError, PftFile, PftHeader,
};

// Re-export the entire file module for advanced usage
#[doc(inline)]
pub use crate::dvine_types::file;
