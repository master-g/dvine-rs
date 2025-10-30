//! Prelude module for `dvine_types`.
//!
//! This module provides a convenient way to import commonly used types, traits, and constants.
//!
//! # Examples
//!
//! ```rust
//! use dvine_types::prelude::*;
//!
//! // Now you can use all common types directly
//! let dsk = DskFile::new();
//! let pft = PftFile::empty();
//! let font = FntFile::new(FontSize::FS16x16);
//! ```

// File module types
#[doc(inline)]
pub use crate::file::{
	// Constants
	BLOCK_SIZE,
	DskError,

	// DSK types
	DskFile,
	Entry,
	FntError,

	// FNT types
	FntFile,
	PftError,

	// PFT types
	PftFile,
	PftHeader,
	// Startup INI types
	StartupIni,
	StartupOpeningMode,
	StartupRenderMode,

	StartupVgaMode,
};

// Font types
#[doc(inline)]
pub use crate::file::fnt::{FontSize, GlyphIter};

#[doc(inline)]
pub use crate::file::fnt::glyph::{Glyph, GlyphBitmap, GlyphBitmapLineIterator};

// Re-export the file module for advanced usage
#[doc(inline)]
pub use crate::file;
