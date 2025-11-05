//! Font file type support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating font files used in the `D+VINE[LUV]`
//! visual novel engine. The font files use a custom format with Shift-JIS character encoding.
//!
//! # Shift-JIS Encoding
//!
//! Shift-JIS is a variable-length character encoding for Japanese text:
//! - **Single-byte characters (1 byte):**
//!   - `0x00-0x7F`: ASCII characters
//!   - `0xA1-0xDF`: Half-width katakana
//! - **Double-byte characters (2 bytes):**
//!   - First byte: `0x81-0x9F` or `0xE0-0xFC`
//!   - Second byte: `0x40-0x7E` or `0x80-0xFC`
//!
//! # Usage Examples
//!
//! ## Basic Character Lookup
//!
//! ```no_run
//! use dvine_types::file::fnt::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let font = File::open("SYSTEM.FNT")?;
//!
//! // Lookup by pre-combined character code
//! let glyph_a = font.lookup(0x0041); // ASCII 'A'
//! let glyph_hiragana = font.lookup(0x82A0); // Hiragana 'あ'
//! # Ok(())
//! # }
//! ```
//!
//! ## Lookup from Shift-JIS Bytes
//!
//! ```no_run
//! use dvine_types::file::fnt::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let font = File::open("SYSTEM.FNT")?;
//!
//! // Single-byte character
//! let (glyph, consumed) = font.lookup_from_bytes(b"A");
//! assert_eq!(consumed, 1);
//!
//! // Double-byte character (あ = 0x82 0xA0 in Shift-JIS)
//! let (glyph, consumed) = font.lookup_from_bytes(&[0x82, 0xA0]);
//! assert_eq!(consumed, 2);
//! # Ok(())
//! # }
//! ```
//!
//! ## Lookup from Byte Stream
//!
//! ```no_run
//! use dvine_types::file::fnt::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let font = File::open("SYSTEM.FNT")?;
//!
//! // Mixed ASCII and Japanese text in Shift-JIS encoding
//! let bytes = b"Hello\x82\xa0"; // "Helloあ"
//! let glyphs = font.lookup_from_stream(bytes);
//!
//! println!("Found {} glyphs", glyphs.len());
//! # Ok(())
//! # }
//! ```

use std::{fmt::Display, io::Cursor};

use crate::file::{DvFileError, FileType, fnt::glyph::Glyph};

pub mod glyph;

/// Font file constants.
pub mod constants {
	/// Size of the font file header in bytes
	pub const HEADER_SIZE: usize = 4;

	/// Offset table size in bytes (14848 entries * 2 bytes each)
	pub const OFFSET_TABLE_SIZE: usize = 0x7400;

	/// Offset table entry count
	pub const OFFSET_TABLE_ENTRIES: usize = 0x3A00;

	/// Bitmap data offset in bytes
	pub const BITMAP_DATA_OFFSET: usize = 0x7404;
}

/// Font size enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FontSize {
	/// 8x8 pixels
	FS8x8 = 8,

	/// 16x16 pixels
	FS16x16 = 16,

	/// 24x24 pixels
	FS24x24 = 24,
}

impl Display for FontSize {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FontSize::FS8x8 => write!(f, "8x8"),
			FontSize::FS16x16 => write!(f, "16x16"),
			FontSize::FS24x24 => write!(f, "24x24"),
		}
	}
}

impl FontSize {
	/// Returns the number of bytes per glyph based on the font size.
	pub fn bytes_per_glyph(&self) -> usize {
		match self {
			FontSize::FS8x8 => 8 * 8 / 8,
			FontSize::FS16x16 => 16 * 16 / 8,
			FontSize::FS24x24 => 24 * 24 / 8,
		}
	}

	/// Returns the number of bytes per row based on the font size.
	pub fn bytes_per_row(&self) -> usize {
		match self {
			FontSize::FS8x8 => 1,
			FontSize::FS16x16 => 2,
			FontSize::FS24x24 => 3,
		}
	}
}

/// Font file structure, representing a complete font file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File {
	/// Font size in N * N pixels
	font_size: FontSize,

	/// Font offset table
	offsets: [u16; constants::OFFSET_TABLE_ENTRIES],

	/// Glyphs data
	raw: Vec<u8>,
}

impl File {
	/// Converts a Shift-JIS character code to an offset table index.
	///
	/// # Arguments
	///
	/// * `code` - Shift-JIS character code
	///
	/// # Returns
	///
	/// The index into the offset table, or None if the code is invalid.
	///
	/// # Algorithm
	///
	/// This implements the exact transformation from the original game engine assembly code.
	/// The algorithm applies two conditional additions with 16-bit wraparound:
	///
	/// 1. If code >= 0xE000: add 0x4000
	/// 2. If code >= 0x8100: add 0x8000
	/// 3. Mask result to 16 bits (allows wraparound)
	///
	/// This maps Shift-JIS codes into the offset table's index space through modulo arithmetic.
	/// For example: 0x82A0 ('あ') + 0x8000 = 0x102A0, masked to 0x02A0.
	fn code_to_index(code: u16) -> Option<usize> {
		let mut index = code as usize;

		// Apply encoding transformations from original assembly code (sub_486790)
		// These transformations use 16-bit wraparound to compress the Shift-JIS
		// code space into the offset table's range
		if code >= 0xE000 {
			index = index.wrapping_add(0x4000);
		}
		if code >= 0x8100 {
			index = index.wrapping_add(0x8000);
		}

		// Mask to 16 bits to handle wraparound (equivalent to assembly "and eax, 0FFFFh")
		index &= 0xFFFF;

		// Validate that index is within offset table range
		if index < constants::OFFSET_TABLE_ENTRIES {
			Some(index)
		} else {
			None
		}
	}
	/// Creates a new Font File instance with the specified font size.
	pub fn new(font_size: FontSize) -> Self {
		Self {
			font_size,
			offsets: [0; constants::OFFSET_TABLE_ENTRIES],
			raw: Vec::new(),
		}
	}

	/// Opens a font file from the specified path.
	///
	/// # Arguments
	///
	/// * `path` - Path to the font file.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The font size is invalid
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let mut file = std::fs::File::open(path)?;
		Self::from_reader(&mut file)
	}

	/// Returns the font size of the font file.
	pub fn font_size(&self) -> FontSize {
		self.font_size
	}

	/// Returns the number of bytes per glyph based on the font size.
	pub fn bytes_per_glyph(&self) -> usize {
		self.font_size.bytes_per_glyph()
	}

	/// Returns the number of glyphs present in the font file.
	pub fn num_of_glyphs(&self) -> usize {
		self.offsets.iter().filter(|&&offset| offset != 0).count()
	}

	/// Returns the offset value at the given index in the offset table.
	///
	/// # Arguments
	///
	/// * `index` - Index into the offset table (0..14848)
	///
	/// # Returns
	///
	/// The offset value, or None if the index is out of range.
	pub fn get_offset(&self, index: usize) -> Option<u16> {
		self.offsets.get(index).copied()
	}

	/// Looks up a glyph by its character code.
	///
	/// # Arguments
	///
	/// * `code` - Character code (Shift-JIS encoding).
	///
	/// # Notes
	///
	/// This function converts Shift-JIS character codes to offset table indices.
	/// The offset value from the table is then multiplied by `bytes_per_glyph` to get
	/// the actual byte offset in the raw data.
	pub fn lookup(&self, code: u16) -> Option<Glyph> {
		// Convert Shift-JIS code to offset table index
		let index = Self::code_to_index(code)?;

		// Get offset from table (this is a glyph number, not byte offset)
		let offset_multiplier = self.offsets[index] as usize;
		if offset_multiplier == 0 {
			return None; // Glyph not present
		}

		// Calculate actual byte offset: offset_multiplier * bytes_per_glyph
		// Note: The assembly code directly multiplies the offset value without subtracting 1
		let bytes_per_glyph = self.bytes_per_glyph();
		let start = offset_multiplier * bytes_per_glyph;
		let end = start + bytes_per_glyph;

		if end > self.raw.len() {
			return None; // Invalid offset
		}

		let data = self.raw[start..end].to_vec();
		Some(Glyph::new(self.font_size, code, data))
	}

	/// Looks up a glyph from Shift-JIS encoded bytes.
	///
	/// # Arguments
	///
	/// * `bytes` - Shift-JIS encoded bytes (1 or 2 bytes)
	///
	/// # Returns
	///
	/// A tuple of `(Option<Glyph>, usize)` where:
	/// - The `Option<Glyph>` is the found glyph (or None if not found)
	/// - The `usize` is the number of bytes consumed (1 or 2)
	///
	/// # Examples
	///
	/// ```no_run
	/// # use dvine_types::file::fnt::{File, FontSize};
	/// # let font = File::new(FontSize::FS16x16);
	/// // Single-byte ASCII character
	/// let (glyph, consumed) = font.lookup_from_bytes(b"A");
	/// assert_eq!(consumed, 1);
	///
	/// // Double-byte Japanese character (あ = 0x82 0xA0)
	/// let (glyph, consumed) = font.lookup_from_bytes(&[0x82, 0xA0]);
	/// assert_eq!(consumed, 2);
	/// ```
	pub fn lookup_from_bytes(&self, bytes: &[u8]) -> (Option<Glyph>, usize) {
		if bytes.is_empty() {
			return (None, 0);
		}

		let first_byte = bytes[0];

		// Determine if this is a single-byte or double-byte character
		// Shift-JIS single-byte ranges:
		// - 0x00-0x7F: ASCII
		// - 0xA1-0xDF: Half-width katakana
		// Double-byte first byte ranges:
		// - 0x81-0x9F, 0xE0-0xFC
		let (code, bytes_consumed) = if first_byte < 0x80 || (0xA1..=0xDF).contains(&first_byte) {
			// Single-byte character
			(first_byte as u16, 1)
		} else if bytes.len() >= 2 {
			// Double-byte character
			// Combine bytes in big-endian order (high byte first)
			let code = u16::from_be_bytes([bytes[0], bytes[1]]);
			(code, 2)
		} else {
			// Incomplete double-byte sequence
			return (None, 1);
		};

		(self.lookup(code), bytes_consumed)
	}

	/// Looks up multiple glyphs from a Shift-JIS encoded byte stream.
	///
	/// # Arguments
	///
	/// * `bytes` - Shift-JIS encoded byte stream
	///
	/// # Returns
	///
	/// A vector of glyphs found in the byte stream. Missing glyphs are skipped.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use dvine_types::file::fnt::{File, FontSize};
	/// # let font = File::new(FontSize::FS16x16);
	/// // Lookup glyphs from mixed ASCII and Japanese text
	/// let bytes = b"Hello\x82\xa0"; // "Helloあ" in Shift-JIS
	/// let glyphs = font.lookup_from_stream(bytes);
	/// ```
	pub fn lookup_from_stream(&self, bytes: &[u8]) -> Vec<Glyph> {
		let mut glyphs = Vec::new();
		let mut i = 0;

		while i < bytes.len() {
			let (glyph, consumed) = self.lookup_from_bytes(&bytes[i..]);
			if consumed == 0 {
				break; // Avoid infinite loop
			}
			if let Some(g) = glyph {
				glyphs.push(g);
			}
			i += consumed;
		}

		glyphs
	}

	/// Inserts a glyph into the font file.
	///
	/// # Arguments
	///
	/// * `glyph` - The glyph to insert.
	/// * `overwrite` - Whether to overwrite an existing glyph.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The character code is out of range
	/// - The glyph already exists and `overwrite` is false
	/// - The glyph size doesn't match the font file's size
	/// - The glyph data length is incorrect
	pub fn insert(&mut self, glyph: &Glyph, overwrite: bool) -> Result<(), DvFileError> {
		// Convert Shift-JIS code to offset table index
		let index = Self::code_to_index(glyph.code()).ok_or(DvFileError::CodeOutOfRange {
			file_type: FileType::Fnt,
			code: glyph.code(),
			max_code: 0xFFFF,
		})?;

		// Validate glyph size matches font file size
		if glyph.font_size() != self.font_size {
			return Err(DvFileError::InvalidFontSize {
				file_type: FileType::Fnt,
				value: glyph.font_size() as u32,
			});
		}

		// Validate glyph data length
		let bytes_per_glyph = self.bytes_per_glyph();
		if glyph.data().len() != bytes_per_glyph {
			return Err(DvFileError::insufficient_data(
				FileType::Fnt,
				bytes_per_glyph,
				glyph.data().len(),
			));
		}

		let offset_multiplier = self.offsets[index];
		if offset_multiplier != 0 && !overwrite {
			// Glyph already exists and overwrite is false
			return Err(DvFileError::GlyphAlreadyExists {
				file_type: FileType::Fnt,
				code: glyph.code(),
			});
		}

		if offset_multiplier != 0 {
			// Overwrite existing glyph data
			let start = (offset_multiplier as usize) * bytes_per_glyph;
			let end = start + bytes_per_glyph;
			self.raw[start..end].copy_from_slice(glyph.data());
		} else {
			// Insert new glyph at the end
			// Note: Offset 0 is reserved for "no glyph", valid offsets start from 1
			// The offset value N points to byte position N * bytes_per_glyph
			// This means offset 1 → byte 32, offset 2 → byte 64, etc.
			// The first 32 bytes (glyph slot 0) are padding/unused

			// If this is the very first glyph, add padding first
			if self.raw.is_empty() {
				self.raw.extend_from_slice(&vec![0u8; bytes_per_glyph]);
			}

			// Calculate new offset: current number of glyph slots in raw data
			// raw.len() / bytes_per_glyph gives us the next available slot number
			let new_offset_multiplier = (self.raw.len() / bytes_per_glyph) as u16;
			self.raw.extend_from_slice(glyph.data());
			self.offsets[index] = new_offset_multiplier;
		}

		Ok(())
	}

	/// Returns an iterator over all glyphs in the font file.
	pub fn iter(&self) -> GlyphIter<'_> {
		GlyphIter {
			file: self,
			current_code: 0,
		}
	}

	/// Serializes the font file to bytes.
	pub fn to_bytes(&self) -> Vec<u8> {
		let total_size = constants::HEADER_SIZE + constants::OFFSET_TABLE_SIZE + self.raw.len();
		let mut buffer = Vec::with_capacity(total_size);

		// Write font size header
		buffer.extend_from_slice(&(self.font_size as u32).to_le_bytes());

		// Write offset table
		for offset in &self.offsets {
			buffer.extend_from_slice(&offset.to_le_bytes());
		}

		// Write bitmap data
		buffer.extend_from_slice(&self.raw);

		buffer
	}

	/// Loads a font file from a byte slice.
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let mut cursor = Cursor::new(data);
		Self::from_reader(&mut cursor)
	}

	/// Loads a font file from any reader.
	pub fn from_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, DvFileError> {
		// Read font size from header
		let mut buf = [0u8; constants::HEADER_SIZE];
		reader.read_exact(&mut buf)?;
		let font_size = match u32::from_le_bytes(buf) {
			8 => FontSize::FS8x8,
			16 => FontSize::FS16x16,
			24 => FontSize::FS24x24,
			other => {
				return Err(DvFileError::InvalidFontSize {
					file_type: FileType::Fnt,
					value: other,
				});
			}
		};

		// Read offset table
		let mut offset_buf = [0u8; constants::OFFSET_TABLE_SIZE];
		reader.read_exact(&mut offset_buf)?;
		let mut offsets = [0u16; constants::OFFSET_TABLE_ENTRIES];
		for (i, offset) in offsets.iter_mut().enumerate() {
			let start = i * 2;
			*offset = u16::from_le_bytes([offset_buf[start], offset_buf[start + 1]]);
		}

		// Read bitmap data
		let mut raw = Vec::new();
		reader.read_to_end(&mut raw)?;

		Ok(Self {
			font_size,
			offsets,
			raw,
		})
	}
}

/// Iterator over glyphs in a font file.
#[derive(Debug)]
pub struct GlyphIter<'a> {
	file: &'a File,
	current_code: u16,
}

impl<'a> Iterator for GlyphIter<'a> {
	type Item = Glyph;

	fn next(&mut self) -> Option<Self::Item> {
		while self.current_code < constants::OFFSET_TABLE_ENTRIES as u16 {
			let code = self.current_code;
			self.current_code += 1;

			if let Some(glyph) = self.file.lookup(code) {
				return Some(glyph);
			}
		}
		None
	}
}

impl<'a> IntoIterator for &'a File {
	type Item = Glyph;
	type IntoIter = GlyphIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Font File: size={}, glyphs={}", self.font_size, self.num_of_glyphs())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_code_to_index_ascii() {
		// ASCII characters - no transformation
		assert_eq!(File::code_to_index(0x0020), Some(0x0020)); // Space
		assert_eq!(File::code_to_index(0x0041), Some(0x0041)); // 'A'
		assert_eq!(File::code_to_index(0x007A), Some(0x007A)); // 'z'
	}

	#[test]
	fn test_code_to_index_half_width_katakana() {
		// Half-width katakana - no transformation
		assert_eq!(File::code_to_index(0x00A0), Some(0x00A0));
		assert_eq!(File::code_to_index(0x00DF), Some(0x00DF));
	}

	#[test]
	fn test_code_to_index_below_0x8100() {
		// Codes below 0x8100 - no transformation
		// Note: 0x8000 (32768) and 0x80FF (33023) exceed the offset table size (0x3A00 = 14848)
		// so they will return None
		assert_eq!(File::code_to_index(0x0080), Some(0x0080)); // Valid low code
		assert_eq!(File::code_to_index(0x00FF), Some(0x00FF)); // Valid low code
		assert_eq!(File::code_to_index(0x1000), Some(0x1000)); // Valid mid-range code
	}

	#[test]
	fn test_code_to_index_double_byte_wraparound() {
		// Double-byte >= 0x8100 - add 0x8000 with 16-bit wraparound
		// 0x8100 + 0x8000 = 0x10100, masked to 16 bits = 0x0100
		assert_eq!(File::code_to_index(0x8100), Some(0x0100));

		// 'あ' (Hiragana A): 0x82A0 + 0x8000 = 0x102A0 → 0x02A0
		assert_eq!(File::code_to_index(0x82A0), Some(0x02A0));

		// '　' (Full-width space): 0x8140 + 0x8000 = 0x10140 → 0x0140
		assert_eq!(File::code_to_index(0x8140), Some(0x0140));

		// 'え': 0x889F + 0x8000 = 0x1089F → 0x089F
		assert_eq!(File::code_to_index(0x889F), Some(0x089F));
	}

	#[test]
	fn test_code_to_index_high_range() {
		// High range >= 0xE000 - add both 0x4000 and 0x8000 (total 0xC000)
		// 0xE000 + 0x4000 + 0x8000 = 0x1A000, masked to 16 bits = 0xA000
		// However, 0xA000 (40960) exceeds offset table size (0x3A00 = 14848)
		// So these return None
		assert_eq!(File::code_to_index(0xE000), None);
		assert_eq!(File::code_to_index(0xFFFF), None);
		assert_eq!(File::code_to_index(0xE040), None);

		// Test that the transformation logic is correct even if out of range
		// The transformed value would be 0xA000 if the table were large enough
		let code = 0xE000u16;
		let mut index = code as usize;
		if code >= 0xE000 {
			index = index.wrapping_add(0x4000);
		}
		if code >= 0x8100 {
			index = index.wrapping_add(0x8000);
		}
		index &= 0xFFFF;
		assert_eq!(index, 0xA000);
	}

	#[test]
	fn test_code_to_index_boundary_conditions() {
		// Test exact boundaries
		// 0x80FF (33023) exceeds table size, returns None
		assert_eq!(File::code_to_index(0x80FF), None);

		// 0x8100 + 0x8000 = 0x10100 → 0x0100 (256) - within range
		assert_eq!(File::code_to_index(0x8100), Some(0x0100));

		// 0xDFFF + 0x8000 = 0x15FFF → 0x5FFF (24575) - exceeds 0x3A00 (14848)
		assert_eq!(File::code_to_index(0xDFFF), None);

		// 0xE000 transforms to 0xA000 (40960) - exceeds table size
		assert_eq!(File::code_to_index(0xE000), None);

		// Test some codes that ARE within range
		assert_eq!(File::code_to_index(0x0000), Some(0x0000));
		assert_eq!(File::code_to_index(0x3000), Some(0x3000));
		assert_eq!(File::code_to_index(0x39FF), Some(0x39FF)); // Just below 0x3A00
	}

	#[test]
	fn test_code_to_index_out_of_range() {
		// After transformations, some indices might exceed offset table size
		// These should return None
		// The offset table has 0x3A00 (14848) entries

		// Direct codes >= 0x3A00 are out of range
		assert_eq!(File::code_to_index(0x3A00), None);
		assert_eq!(File::code_to_index(0x4000), None);
		assert_eq!(File::code_to_index(0x7FFF), None);

		// Codes that transform to values >= 0x3A00 are also out of range
		// 0x8100 + 0x8000 = 0x0100 (OK, within range)
		assert_eq!(File::code_to_index(0x8100), Some(0x0100));

		// 0xBA00 + 0x8000 = 0x13A00 → 0x3A00 (exactly at limit, out of range)
		assert_eq!(File::code_to_index(0xBA00), None);
	}

	#[test]
	fn test_bytes_per_glyph() {
		let font_8 = File::new(FontSize::FS8x8);
		assert_eq!(font_8.bytes_per_glyph(), 8);

		let font_16 = File::new(FontSize::FS16x16);
		assert_eq!(font_16.bytes_per_glyph(), 32);

		let font_24 = File::new(FontSize::FS24x24);
		assert_eq!(font_24.bytes_per_glyph(), 72);
	}

	#[test]
	fn test_glyph_insert_and_lookup() {
		let mut font = File::new(FontSize::FS16x16);

		// Create a test glyph
		let data = vec![0u8; 32]; // 16x16 / 8 = 32 bytes
		let glyph = Glyph::new(FontSize::FS16x16, 0x0041, data); // 'A'

		// Insert the glyph
		assert!(font.insert(&glyph, false).is_ok());

		// Look up the glyph
		let retrieved = font.lookup(0x0041);
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().code(), 0x0041);
	}

	#[test]
	fn test_glyph_overwrite() {
		let mut font = File::new(FontSize::FS16x16);

		let data1 = vec![1u8; 32];
		let glyph1 = Glyph::new(FontSize::FS16x16, 0x0041, data1);
		font.insert(&glyph1, false).unwrap();

		// Try to insert again without overwrite flag - should fail
		let data2 = vec![2u8; 32];
		let glyph2 = Glyph::new(FontSize::FS16x16, 0x0041, data2.clone());
		assert!(font.insert(&glyph2, false).is_err());

		// Insert with overwrite flag - should succeed
		assert!(font.insert(&glyph2, true).is_ok());

		// Verify the data was overwritten
		let retrieved = font.lookup(0x0041).unwrap();
		assert_eq!(retrieved.data(), &data2);
	}

	#[test]
	fn test_serialization_roundtrip() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert some glyphs
		let data = vec![0xAA; 32];
		let glyph = Glyph::new(FontSize::FS16x16, 0x0041, data);
		font.insert(&glyph, false).unwrap();

		// Serialize to bytes
		let bytes = font.to_bytes();

		// Deserialize
		let loaded = File::from_bytes(&bytes).unwrap();

		// Verify
		assert_eq!(loaded.font_size(), font.font_size());
		assert_eq!(loaded.num_of_glyphs(), font.num_of_glyphs());

		let retrieved = loaded.lookup(0x0041);
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().code(), 0x0041);
	}

	#[test]
	fn test_iterator() {
		let mut font = File::new(FontSize::FS8x8);

		// Insert a few glyphs
		let codes = [0x0041, 0x0042, 0x0043]; // A, B, C
		for &code in &codes {
			let data = vec![0u8; 8];
			let glyph = Glyph::new(FontSize::FS8x8, code, data);
			font.insert(&glyph, false).unwrap();
		}

		// Iterate and collect
		let collected: Vec<u16> = font.iter().map(|g| g.code()).collect();

		// Should have exactly 3 glyphs
		assert_eq!(collected.len(), 3);
		assert!(collected.contains(&0x0041));
		assert!(collected.contains(&0x0042));
		assert!(collected.contains(&0x0043));
	}

	#[test]
	#[ignore] // Only run when SYSTEM.FNT is available
	fn test_japanese_characters_real_file() {
		// This test requires the actual SYSTEM.FNT file
		let font_path = std::path::Path::new("bin/SYSTEM.FNT");
		if !font_path.exists() {
			eprintln!("Skipping test: SYSTEM.FNT not found");
			return;
		}

		let font = File::open(font_path).expect("Failed to open SYSTEM.FNT");
		assert_eq!(font.font_size(), FontSize::FS16x16);

		// Test Hiragana characters (all should exist in SYSTEM.FNT)
		let hiragana_tests = [
			(0x82A0, "あ"), // Hiragana A
			(0x82A2, "い"), // Hiragana I
			(0x82A4, "う"), // Hiragana U
			(0x82A6, "え"), // Hiragana E
			(0x82A8, "お"), // Hiragana O
		];

		for (code, name) in hiragana_tests {
			let glyph = font.lookup(code);
			assert!(glyph.is_some(), "Failed to find glyph for '{}' (code 0x{:04X})", name, code);
			assert_eq!(glyph.unwrap().code(), code);
		}

		// Test ASCII (should exist)
		assert!(font.lookup(0x0041).is_some(), "ASCII 'A' should exist");
		assert!(font.lookup(0x0030).is_some(), "ASCII '0' should exist");

		// Test full-width space (might be empty, offset=0)
		let fw_space = font.lookup(0x8140);
		// Don't assert existence, just verify no crash
		println!("Full-width space lookup: {:?}", fw_space.is_some());
	}

	#[test]
	fn test_lookup_from_bytes_single_byte() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert ASCII 'A'
		let data = vec![0xAA; 32];
		let glyph = Glyph::new(FontSize::FS16x16, 0x0041, data.clone());
		font.insert(&glyph, false).unwrap();

		// Test lookup from single byte
		let (result, consumed) = font.lookup_from_bytes(b"A");
		assert_eq!(consumed, 1);
		assert!(result.is_some());
		assert_eq!(result.unwrap().code(), 0x0041);

		// Test half-width katakana (0xA1-0xDF range)
		let data2 = vec![0xBB; 32];
		let glyph2 = Glyph::new(FontSize::FS16x16, 0x00A1, data2);
		font.insert(&glyph2, false).unwrap();

		let (result, consumed) = font.lookup_from_bytes(&[0xA1]);
		assert_eq!(consumed, 1);
		assert!(result.is_some());
		assert_eq!(result.unwrap().code(), 0x00A1);
	}

	#[test]
	fn test_lookup_from_bytes_double_byte() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert Hiragana 'あ' (0x82A0)
		let data = vec![0xCC; 32];
		let glyph = Glyph::new(FontSize::FS16x16, 0x82A0, data.clone());
		font.insert(&glyph, false).unwrap();

		// Test lookup from double bytes
		let (result, consumed) = font.lookup_from_bytes(&[0x82, 0xA0]);
		assert_eq!(consumed, 2);
		assert!(result.is_some());
		assert_eq!(result.unwrap().code(), 0x82A0);
	}

	#[test]
	fn test_lookup_from_bytes_incomplete_sequence() {
		let font = File::new(FontSize::FS16x16);

		// Test incomplete double-byte sequence
		let (result, consumed) = font.lookup_from_bytes(&[0x82]);
		assert_eq!(consumed, 1);
		assert!(result.is_none());
	}

	#[test]
	fn test_lookup_from_bytes_empty() {
		let font = File::new(FontSize::FS16x16);

		// Test empty bytes
		let (result, consumed) = font.lookup_from_bytes(&[]);
		assert_eq!(consumed, 0);
		assert!(result.is_none());
	}

	#[test]
	fn test_lookup_from_bytes_shift_jis_stream() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert some test glyphs
		let codes = [0x0041, 0x82A0, 0x82A2]; // A, あ, い
		for code in codes {
			let data = vec![0; 32];
			let glyph = Glyph::new(FontSize::FS16x16, code, data);
			font.insert(&glyph, false).unwrap();
		}

		// Simulate reading from a Shift-JIS byte stream: "Aあい"
		let bytes = [0x41, 0x82, 0xA0, 0x82, 0xA2];
		let mut i = 0;
		let mut found_codes = Vec::new();

		while i < bytes.len() {
			let (glyph, consumed) = font.lookup_from_bytes(&bytes[i..]);
			if let Some(g) = glyph {
				found_codes.push(g.code());
			}
			i += consumed;
		}

		assert_eq!(found_codes, vec![0x0041, 0x82A0, 0x82A2]);
	}

	#[test]
	fn test_lookup_from_stream() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert some test glyphs
		let codes = [0x0041, 0x0042, 0x82A0, 0x82A2]; // A, B, あ, い
		for code in codes {
			let data = vec![0; 32];
			let glyph = Glyph::new(FontSize::FS16x16, code, data);
			font.insert(&glyph, false).unwrap();
		}

		// Test stream lookup: "ABあい"
		let bytes = vec![0x41, 0x42, 0x82, 0xA0, 0x82, 0xA2];
		let glyphs = font.lookup_from_stream(&bytes);

		assert_eq!(glyphs.len(), 4);
		assert_eq!(glyphs[0].code(), 0x0041);
		assert_eq!(glyphs[1].code(), 0x0042);
		assert_eq!(glyphs[2].code(), 0x82A0);
		assert_eq!(glyphs[3].code(), 0x82A2);
	}

	#[test]
	fn test_lookup_from_stream_with_missing_glyphs() {
		let mut font = File::new(FontSize::FS16x16);

		// Insert only 'A'
		let data = vec![0; 32];
		let glyph = Glyph::new(FontSize::FS16x16, 0x0041, data);
		font.insert(&glyph, false).unwrap();

		// Stream contains A, B (missing), C (missing)
		let bytes = b"ABC";
		let glyphs = font.lookup_from_stream(bytes);

		// Should only find 'A'
		assert_eq!(glyphs.len(), 1);
		assert_eq!(glyphs[0].code(), 0x0041);
	}
}
