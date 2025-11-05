//! Internal crate for `dvine-rs`.
//!
//! This module is separated into its own crate to enable simple dynamic linking for `dvine`,
//! and should not be used directly.
//!
//! # Examples
//!
//! ```rust
//! use dvine_internal::prelude::*;
//! use std::io::Cursor;
//!
//! // All commonly used types are available
//! let pft = PftFile::empty();
//! let dsk = DskFile::new(Cursor::new(vec![]), pft);
//! let font = FntFile::new(FontSize::FS16x16);
//! ```

/// `use dvine_internal::prelude::*;` to import commonly used items.
pub mod prelude;

// Re-export dvine_types for convenience
pub use dvine_types;
