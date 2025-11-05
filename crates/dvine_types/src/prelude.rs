//! Prelude module for `dvine_types`.
//!
//! This module provides a convenient way to import commonly used types, traits, and constants.
//!
//! # Examples
//!
//! ```no_run
//! use dvine_types::prelude::*;
//!
//! // Now you can use all common types directly
//! let font = FntFile::new(FontSize::FS16x16);
//! let mut items = ItemFile::new();
//! ```

// File module types
#[doc(inline)]
pub use crate::file::{
	// Constants
	DSK_BLOCK_SIZE,

	// DSK types
	DskFile,
	DvFileError,

	// EFC types
	EfcFile,

	Entry,
	FileType,

	// FNT types
	FntFile,

	ItemEntry,
	ItemFile,
	// Item types
	ItemRaw,

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
