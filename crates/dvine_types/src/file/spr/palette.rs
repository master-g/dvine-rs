//! SPR palette support.
//!
//! This module provides types for working with SPR color palettes.
//! SPR files use a 80-color palette stored in the SPR.PAL file.

use std::io::Read;
use std::path::Path;

use crate::file::DvFileError;

/// Number of colors in the SPR.PAL file
pub const SPR_PAL_COLOR_COUNT: usize = 80;

/// Size of the SPR.PAL file in bytes (80 colors × 4 bytes)
pub const SPR_PAL_FILE_SIZE: usize = SPR_PAL_COLOR_COUNT * 4;

/// SPR color palette (80 colors + 176 grayscale).
///
/// SPR.PAL file format:
/// - 80 colors × 4 bytes (RGBX format)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
	/// 80-color palette, RGBX format
	colors: [u8; SPR_PAL_FILE_SIZE],
}

impl Palette {
	/// Creates a new empty palette with all colors set to transparent black.
	pub fn new() -> Self {
		Self {
			colors: [0u8; SPR_PAL_FILE_SIZE],
		}
	}

	/// Loads a palette from an SPR.PAL file.
	///
	/// # Arguments
	///
	/// * `path` - Path to the SPR.PAL file
	///
	/// # File Format
	///
	/// The SPR.PAL file contains 80 colors in RGBX format (320 bytes total).
	/// Each color is 4 bytes: R, G, B, X (where X is ignored/padding).
	///
	/// # Returns
	///
	/// A palette with:
	/// - Indices 0-79: Colors from the file
	/// - Indices 80-255: Grayscale gradient
	pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DvFileError> {
		let data = std::fs::read(path)?;
		Self::from_bytes(&data)
	}

	/// Loads a palette from a byte slice.
	///
	/// # Arguments
	///
	/// * `data` - Raw palette data (must be at least 320 bytes)
	///
	/// # Returns
	///
	/// A palette with colors loaded from the data.
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let mut reader = std::io::Cursor::new(data);
		Self::from_reader(&mut reader)
	}

	/// Loads a palette from a reader.
	///
	/// # Arguments
	///
	/// * `reader` - Data reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut colors = [0u8; SPR_PAL_FILE_SIZE];
		reader.read_exact(&mut colors)?;

		Ok(Self {
			colors,
		})
	}

	/// Gets a color by index.
	///
	/// # Arguments
	///
	/// * `index` - Color index (0-255)
	///
	/// # Returns
	///
	/// The color at the specified index, or None if index is out of range.
	#[inline]
	pub fn get(&self, index: u8) -> (u8, u8, u8, u8) {
		let offset = index as usize * 4;
		(
			self.colors[offset],
			self.colors[offset + 1],
			self.colors[offset + 2],
			self.colors[offset + 3],
		)
	}

	/// Sets a color at the specified index.
	///
	/// # Arguments
	///
	/// * `index` - Color index (0-255)
	/// * `color` - New color value
	#[inline]
	pub fn set(&mut self, index: u8, color: (u8, u8, u8, u8)) {
		let offset = index as usize * 4;
		self.colors[offset] = color.0;
		self.colors[offset + 1] = color.1;
		self.colors[offset + 2] = color.2;
		self.colors[offset + 3] = color.3;
	}

	/// Returns a reference to the color array.
	#[inline]
	pub fn colors(&self) -> &[u8; SPR_PAL_FILE_SIZE] {
		&self.colors
	}

	/// Returns a mutable reference to the color array.
	#[inline]
	pub fn colors_mut(&mut self) -> &mut [u8; SPR_PAL_FILE_SIZE] {
		&mut self.colors
	}

	/// Saves the palette to a file in SPR.PAL format.
	///
	/// Only the first 80 colors are saved.
	///
	/// # Arguments
	///
	/// * `path` - Output file path
	pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), DvFileError> {
		std::fs::write(path, self.colors)?;
		Ok(())
	}

	/// Converts the palette to bytes in SPR.PAL format.
	///
	/// Only the first 80 colors are included.
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(SPR_PAL_FILE_SIZE);
		data.extend_from_slice(&self.colors);

		data
	}
}

impl Default for Palette {
	fn default() -> Self {
		Self::new()
	}
}

impl std::ops::Index<u8> for Palette {
	type Output = [u8];

	/// Returns a slice reference to the 4-byte color data (RGBX format).
	///
	/// # Panics
	///
	/// Panics if index is >= 80 (only 80 colors in palette).
	fn index(&self, index: u8) -> &Self::Output {
		let offset = index as usize * 4;
		assert!(index < SPR_PAL_COLOR_COUNT as u8, "Palette index out of bounds: {}", index);
		&self.colors[offset..offset + 4]
	}
}

impl std::ops::IndexMut<u8> for Palette {
	/// Returns a mutable slice reference to the 4-byte color data (RGBX format).
	///
	/// # Panics
	///
	/// Panics if index is >= 80 (only 80 colors in palette).
	fn index_mut(&mut self, index: u8) -> &mut Self::Output {
		let offset = index as usize * 4;
		assert!(index < SPR_PAL_COLOR_COUNT as u8, "Palette index out of bounds: {}", index);
		&mut self.colors[offset..offset + 4]
	}
}
