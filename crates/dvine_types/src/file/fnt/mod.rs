//! Font file type support for `dvine-rs` project.

use crate::file::{error::FntError, fnt::glyph::Glyph};

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
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, FntError> {
		use std::io::Read;

		let mut file = std::fs::File::open(path)?;

		// Read font size from header
		let mut buf = [0u8; constants::HEADER_SIZE];
		file.read_exact(&mut buf)?;
		let font_size = match u32::from_le_bytes(buf) {
			8 => FontSize::FS8x8,
			16 => FontSize::FS16x16,
			24 => FontSize::FS24x24,
			other => return Err(FntError::InvalidFontSize(other)),
		};

		// Read offset table
		let mut offset_buf = [0u8; constants::OFFSET_TABLE_SIZE];
		file.read_exact(&mut offset_buf)?;
		let mut offsets = [0u16; constants::OFFSET_TABLE_ENTRIES];
		for (i, offset) in offsets.iter_mut().enumerate() {
			let start = i * 2;
			*offset = u16::from_le_bytes([offset_buf[start], offset_buf[start + 1]]);
		}

		// Read bitmap data
		let mut raw = Vec::new();
		file.read_to_end(&mut raw)?;

		Ok(Self {
			font_size,
			offsets,
			raw,
		})
	}

	/// Returns the font size of the font file.
	pub fn font_size(&self) -> FontSize {
		self.font_size
	}

	/// Returns the number of bytes per glyph based on the font size.
	pub fn bytes_per_glyph(&self) -> usize {
		self.font_size.bytes_per_glyph()
	}

	/// Looks up a glyph by its character code.
	///
	/// # Arguments
	///
	/// * `code` - Character code (Shift-JIS encoding).
	pub fn lookup(&self, code: u16) -> Option<Glyph> {
		let index = code as usize;
		if index >= self.offsets.len() {
			return None;
		}

		let offset = self.offsets[index] as usize;
		if offset == 0 {
			return None; // Glyph not present
		}

		let bytes_per_glyph = self.bytes_per_glyph();
		let start = offset;
		let end = start + bytes_per_glyph;

		if end > self.raw.len() {
			return None; // Invalid offset
		}

		let data = self.raw[start..end].to_vec();
		Some(Glyph::new(self.font_size, code, data))
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
	pub fn insert(&mut self, glyph: &Glyph, overwrite: bool) -> Result<(), FntError> {
		let index = glyph.code() as usize;
		if index >= self.offsets.len() {
			return Err(FntError::CodeOutOfRange {
				code: glyph.code(),
				max_code: (constants::OFFSET_TABLE_ENTRIES - 1) as u16,
			});
		}

		// Validate glyph size matches font file size
		if glyph.font_size() != self.font_size {
			return Err(FntError::InvalidFontSize(glyph.font_size() as u32));
		}

		// Validate glyph data length
		let bytes_per_glyph = self.bytes_per_glyph();
		if glyph.data().len() != bytes_per_glyph {
			return Err(FntError::InsufficientData {
				expected: bytes_per_glyph,
				actual: glyph.data().len(),
			});
		}

		let offset = self.offsets[index];
		if offset != 0 && !overwrite {
			// Glyph already exists and overwrite is false
			return Err(FntError::GlyphAlreadyExists {
				code: glyph.code(),
			});
		}

		if offset != 0 {
			// Overwrite existing glyph data
			let start = offset as usize;
			let end = start + bytes_per_glyph;
			self.raw[start..end].copy_from_slice(glyph.data());
		} else {
			// Insert new glyph at the end
			let new_offset = self.raw.len() as u16;
			self.raw.extend_from_slice(glyph.data());
			self.offsets[index] = new_offset;
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
	pub fn from_bytes(data: &[u8]) -> Result<Self, FntError> {
		use std::io::{Cursor, Read};

		let mut cursor = Cursor::new(data);

		// Read font size from header
		let mut buf = [0u8; constants::HEADER_SIZE];
		cursor.read_exact(&mut buf)?;
		let font_size = match u32::from_le_bytes(buf) {
			8 => FontSize::FS8x8,
			16 => FontSize::FS16x16,
			24 => FontSize::FS24x24,
			other => return Err(FntError::InvalidFontSize(other)),
		};

		// Read offset table
		let mut offset_buf = [0u8; constants::OFFSET_TABLE_SIZE];
		cursor.read_exact(&mut offset_buf)?;
		let mut offsets = [0u16; constants::OFFSET_TABLE_ENTRIES];
		for (i, offset) in offsets.iter_mut().enumerate() {
			let start = i * 2;
			*offset = u16::from_le_bytes([offset_buf[start], offset_buf[start + 1]]);
		}

		// Read bitmap data
		let mut raw = Vec::new();
		cursor.read_to_end(&mut raw)?;

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

impl TryFrom<&[u8]> for File {
	type Error = FntError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for File {
	type Error = FntError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&Vec<u8>> for File {
	type Error = FntError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<File> for Vec<u8> {
	fn from(file: File) -> Self {
		file.to_bytes()
	}
}

impl From<&File> for Vec<u8> {
	fn from(file: &File) -> Self {
		file.to_bytes()
	}
}
