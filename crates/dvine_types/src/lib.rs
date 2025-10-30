//! This crate provides core data types and file format support for the `dvine-rs` project.
//!
//! # File Formats
//!
//! - **DSK**: Block-based container files that store multiple files in 2048-byte blocks
//! - **PFT**: Pack File Table containing metadata for files in DSK containers
//! - **FNT**: Font files with various glyph sizes (8x8, 16x16, 24x24)
//! - **`StartupIni`**: Configuration file defining startup parameters for games
//!
//! # Examples
//!
//! Using the prelude (recommended):
//!
//! ```rust
//! use dvine_types::prelude::*;
//!
//! // Create a new DSK/PFT pair
//! let mut dsk = DskFile::new();
//! let data = b"Hello, World!";
//! let (index, size) = dsk.add_file(data);
//!
//! let entry = Entry::new("readme.txt", index, size);
//! let pft = PftFile::new(vec![entry]);
//!
//! // Work with fonts
//! let font = FntFile::new(FontSize::FS16x16);
//! ```
//!
//! Or use explicit paths:
//!
//! ```rust
//! use dvine_types::file::{DskFile, PftFile, PftEntry as Entry};
//!
//! let mut dsk = DskFile::new();
//! // ...
//! ```

pub mod file;

/// `use dvine_types::prelude::*;` to import commonly used items.
pub mod prelude;
