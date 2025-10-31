//! File type support for `dvine-rs` project.

mod error;

pub mod dsk;
pub mod fnt;
pub mod item;
pub mod pft;
pub mod startup_ini;

/// Block size used in DSK files (2048 bytes / 0x0800)
pub const BLOCK_SIZE: usize = 0x0800;

// Re-export error types
pub use error::{DskError, FntError, ItemError, PftError};

// Re-export main file types
pub use dsk::File as DskFile;
pub use fnt::{
	File as FntFile, FontSize, GlyphIter, glyph::Glyph, glyph::GlyphBitmap,
	glyph::GlyphBitmapLineIterator,
};
pub use item::{File as ItemFile, ItemRaw, entry::ItemEntry};
pub use pft::{Entry, File as PftFile, Header as PftHeader};
pub use startup_ini::{
	OpeningMode as StartupOpeningMode, RenderMode as StartupRenderMode, StartupIni,
	VgaMode as StartupVgaMode,
};
