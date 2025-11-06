//! SPR palette support.
//!
//! This module provides types for working with SPR color palettes.
//! SPR files use a 80-color palette stored in the SPR.PAL file.

use std::fmt;
use std::io::Read;
use std::path::Path;

use crate::file::DvFileError;

/// RGBA color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
	/// Red component (0-255)
	pub r: u8,
	/// Green component (0-255)
	pub g: u8,
	/// Blue component (0-255)
	pub b: u8,
	/// Alpha component (0-255)
	pub a: u8,
}

impl Color {
	/// Creates a new RGBA color.
	///
	/// # Arguments
	///
	/// * `r` - Red component (0-255)
	/// * `g` - Green component (0-255)
	/// * `b` - Blue component (0-255)
	/// * `a` - Alpha component (0-255)
	pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
		Self {
			r,
			g,
			b,
			a,
		}
	}

	/// Creates a new RGB color with full opacity.
	pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
		Self::new(r, g, b, 255)
	}

	/// Creates a new grayscale color.
	pub const fn gray(value: u8) -> Self {
		Self::rgb(value, value, value)
	}

	/// Creates a transparent black color.
	pub const fn transparent() -> Self {
		Self::new(0, 0, 0, 0)
	}

	/// Returns the color as a 32-bit RGBA value.
	pub const fn to_rgba32(&self) -> u32 {
		((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
	}

	/// Creates a color from a 32-bit RGBA value.
	pub const fn from_rgba32(rgba: u32) -> Self {
		Self {
			r: ((rgba >> 24) & 0xFF) as u8,
			g: ((rgba >> 16) & 0xFF) as u8,
			b: ((rgba >> 8) & 0xFF) as u8,
			a: (rgba & 0xFF) as u8,
		}
	}
}

impl Default for Color {
	fn default() -> Self {
		Self::transparent()
	}
}

impl fmt::Display for Color {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "RGBA({}, {}, {}, {})", self.r, self.g, self.b, self.a)
	}
}

/// SPR color palette (80 colors + 176 grayscale).
///
/// SPR.PAL file format:
/// - 80 colors × 4 bytes (RGBX format)
/// - Remaining indices (80-255) are filled with grayscale values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
	/// 256-color palette
	colors: [Color; 256],
}

impl Palette {
	/// Number of colors in the SPR.PAL file
	pub const SPR_PAL_COLOR_COUNT: usize = 80;

	/// Size of the SPR.PAL file in bytes (80 colors × 4 bytes)
	pub const SPR_PAL_FILE_SIZE: usize = Self::SPR_PAL_COLOR_COUNT * 4;

	/// Total palette size
	pub const PALETTE_SIZE: usize = 256;

	/// Creates a new empty palette with all colors set to transparent black.
	pub fn new() -> Self {
		Self {
			colors: [Color::transparent(); 256],
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
		let mut palette = Self::new();

		// Read 80 colors in RGBX format
		for i in 0..Self::SPR_PAL_COLOR_COUNT {
			let mut rgbx = [0u8; 4];
			reader.read_exact(&mut rgbx)?;

			palette.colors[i] = Color::new(
				rgbx[0], // R
				rgbx[1], // G
				rgbx[2], // B
				255,     // Fully opaque
			);
		}

		// Fill remaining colors with grayscale gradient
		for i in Self::SPR_PAL_COLOR_COUNT..Self::PALETTE_SIZE {
			palette.colors[i] = Color::gray(i as u8);
		}

		Ok(palette)
	}

	/// Creates a default grayscale palette.
	///
	/// All 256 colors are set to grayscale values matching their index.
	pub fn grayscale() -> Self {
		let mut palette = Self::new();
		for i in 0..Self::PALETTE_SIZE {
			palette.colors[i] = Color::gray(i as u8);
		}
		palette
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
	pub fn get(&self, index: u8) -> Color {
		self.colors[index as usize]
	}

	/// Sets a color at the specified index.
	///
	/// # Arguments
	///
	/// * `index` - Color index (0-255)
	/// * `color` - New color value
	#[inline]
	pub fn set(&mut self, index: u8, color: Color) {
		self.colors[index as usize] = color;
	}

	/// Returns a reference to the color array.
	#[inline]
	pub fn colors(&self) -> &[Color; 256] {
		&self.colors
	}

	/// Returns a mutable reference to the color array.
	#[inline]
	pub fn colors_mut(&mut self) -> &mut [Color; 256] {
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
		let mut data = Vec::with_capacity(Self::SPR_PAL_FILE_SIZE);

		// Write first 80 colors in RGBX format
		for i in 0..Self::SPR_PAL_COLOR_COUNT {
			let color = self.colors[i];
			data.push(color.r);
			data.push(color.g);
			data.push(color.b);
			data.push(0); // X padding
		}

		std::fs::write(path, data)?;
		Ok(())
	}

	/// Converts the palette to bytes in SPR.PAL format.
	///
	/// Only the first 80 colors are included.
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(Self::SPR_PAL_FILE_SIZE);

		for i in 0..Self::SPR_PAL_COLOR_COUNT {
			let color = self.colors[i];
			data.push(color.r);
			data.push(color.g);
			data.push(color.b);
			data.push(0); // X padding
		}

		data
	}

	/// Returns an iterator over palette colors.
	pub fn iter(&self) -> impl Iterator<Item = &Color> {
		self.colors.iter()
	}

	/// Returns an iterator over palette colors with indices.
	pub fn iter_indexed(&self) -> impl Iterator<Item = (u8, &Color)> {
		self.colors.iter().enumerate().map(|(i, c)| (i as u8, c))
	}
}

impl Default for Palette {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Display for Palette {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "SPR Palette: {} colors defined", Self::SPR_PAL_COLOR_COUNT)
	}
}

impl std::ops::Index<u8> for Palette {
	type Output = Color;

	fn index(&self, index: u8) -> &Self::Output {
		&self.colors[index as usize]
	}
}

impl std::ops::IndexMut<u8> for Palette {
	fn index_mut(&mut self, index: u8) -> &mut Self::Output {
		&mut self.colors[index as usize]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_color_creation() {
		let color = Color::new(255, 128, 64, 255);
		assert_eq!(color.r, 255);
		assert_eq!(color.g, 128);
		assert_eq!(color.b, 64);
		assert_eq!(color.a, 255);
	}

	#[test]
	fn test_color_rgb() {
		let color = Color::rgb(255, 128, 64);
		assert_eq!(color.a, 255);
	}

	#[test]
	fn test_color_gray() {
		let color = Color::gray(128);
		assert_eq!(color.r, 128);
		assert_eq!(color.g, 128);
		assert_eq!(color.b, 128);
		assert_eq!(color.a, 255);
	}

	#[test]
	fn test_palette_new() {
		let palette = Palette::new();
		assert_eq!(palette.get(0), Color::transparent());
	}

	#[test]
	fn test_palette_grayscale() {
		let palette = Palette::grayscale();
		assert_eq!(palette.get(0), Color::gray(0));
		assert_eq!(palette.get(128), Color::gray(128));
		assert_eq!(palette.get(255), Color::gray(255));
	}

	#[test]
	fn test_palette_get_set() {
		let mut palette = Palette::new();
		let color = Color::rgb(255, 128, 64);

		palette.set(42, color);
		assert_eq!(palette.get(42), color);
	}

	#[test]
	fn test_palette_index() {
		let mut palette = Palette::new();
		let color = Color::rgb(255, 128, 64);

		palette[42] = color;
		assert_eq!(palette[42], color);
	}

	#[test]
	fn test_palette_from_bytes() {
		let mut data = vec![0u8; Palette::SPR_PAL_FILE_SIZE];

		// Set first color to red
		data[0] = 255; // R
		data[1] = 0; // G
		data[2] = 0; // B
		data[3] = 0; // X

		let palette = Palette::from_bytes(&data).unwrap();
		assert_eq!(palette.get(0), Color::rgb(255, 0, 0));

		// Check grayscale fill for remaining colors
		assert_eq!(palette.get(80), Color::gray(80));
		assert_eq!(palette.get(255), Color::gray(255));
	}

	#[test]
	fn test_palette_roundtrip() {
		let mut original = Palette::new();
		original.set(0, Color::rgb(255, 0, 0));
		original.set(10, Color::rgb(0, 255, 0));
		original.set(20, Color::rgb(0, 0, 255));

		let bytes = original.to_bytes();
		let loaded = Palette::from_bytes(&bytes).unwrap();

		assert_eq!(loaded.get(0), Color::rgb(255, 0, 0));
		assert_eq!(loaded.get(10), Color::rgb(0, 255, 0));
		assert_eq!(loaded.get(20), Color::rgb(0, 0, 255));
	}
}
