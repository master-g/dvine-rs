//! This crate provides core data types and file format support for the `dvine-rs` project.
//!
//! # File Formats
//!
//! - **DSK**: Block-based container files that store multiple files in 2048-byte blocks
//! - **PFT**: Pack File Table containing metadata for files in DSK containers
//! - **FNT**: Font files with various glyph sizes (8x8, 16x16, 24x24)
//! - **ITEM**: Item database files with encrypted data and checksum validation
//! - **`StartupIni`**: Configuration file defining startup parameters for games
//!
//! # Examples
//!
//! Using the prelude (recommended):
//!
//! ```no_run
//! use dvine_types::prelude::*;
//!
//! // Work with fonts
//! let font = FntFile::new(FontSize::FS16x16);
//!
//! // Work with items (use 208-byte arrays)
//! let mut items = ItemFile::new();
//! let item_data = [0u8; 208]; // Item data placeholder
//! items.add_item(item_data);
//! ```
//!
//! Or use explicit paths:
//!
//! ```no_run
//! use dvine_types::file::{ItemFile, FntFile, FontSize};
//!
//! let mut items = ItemFile::new();
//! // ...
//! ```

pub mod file;

/// `use dvine_types::prelude::*;` to import commonly used items.
pub mod prelude;
