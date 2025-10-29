//! This crate provides core data types and file format support for the `dvine-rs` project,
//!
//! # File Formats
//!
//! - **DSK**: Block-based container files that store multiple files in 2048-byte blocks
//! - **PFT**: Pack File Table containing metadata for files in DSK containers
//! - **`StartupIni`**: Configuration file defining startup parameters for games
//!
//! # Examples
//!
//! ```rust
//! use dvine_types::file::{DskFile, PftFile, Entry, BLOCK_SIZE};
//!
//! // Create a new DSK/PFT pair
//! let mut dsk = DskFile::new();
//! let data = b"Hello, World!";
//! let (index, size) = dsk.add_file(data);
//!
//! let entry = Entry::new("readme.txt", index, size);
//! let pft = PftFile::new(vec![entry]);
//! ```

pub mod file;

// Re-export commonly used file types at crate root for convenience
pub use file::{
	BLOCK_SIZE, DskError, DskFile, Entry, PftError, PftFile, PftHeader, StartupIni,
	StartupOpeningMode, StartupRenderMode, StartupVgaMode,
};
