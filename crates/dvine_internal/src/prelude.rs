//! Prelude module for `dvine_internal`.
//!
//! This module provides a convenient way to import commonly used types and traits.
//!
//! # Examples
//!
//! ```rust
//! use dvine_internal::prelude::*;
//!
//! // Now you can use all common types directly
//! let dsk = DskFile::new();
//! let pft = PftFile::empty();
//! let font = FntFile::new(FontSize::FS16x16);
//!
//! // Work with glyphs
//! let glyph = Glyph::blank(0x0041, FontSize::FS8x8);
//! let bitmap: GlyphBitmap = (&glyph).into();
//! ```

// Re-export everything from dvine_types::prelude
#[doc(inline)]
pub use dvine_types::prelude::*;

// Re-export the entire dvine_types module for advanced usage
#[doc(inline)]
pub use dvine_types;
