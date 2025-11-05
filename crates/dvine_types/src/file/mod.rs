//! File type support for `dvine-rs` project.

mod error;

pub mod dsk;
pub mod efc;
pub mod fnt;
pub mod item;
pub mod kg;
pub mod pft;
pub mod startup_ini;

/// Block size used in DSK files (2048 bytes / 0x0800)
pub const DSK_BLOCK_SIZE: usize = 0x0800;

// Re-export unified error type
pub use error::{DvFileError, FileType};

// Re-export main file types
pub use dsk::File as DskFile;
pub use efc::{
	AdpcmDataHeader, DecodedSound, EffectInfo, File as EfcFile, FileBuilder as EfcFileBuilder,
	SoundDataHeader,
};
pub use fnt::{
	File as FntFile, FontSize, GlyphIter, glyph::Glyph, glyph::GlyphBitmap,
	glyph::GlyphBitmapLineIterator,
};
pub use item::{File as ItemFile, ItemRaw, entry::ItemEntry};
pub use kg::{Compression as KgCompressionType, File as KgFile, Header as KgHeader};
pub use pft::{Entry, File as PftFile, Header as PftHeader};
pub use startup_ini::{
	OpeningMode as StartupOpeningMode, RenderMode as StartupRenderMode, StartupIni,
	VgaMode as StartupVgaMode,
};
