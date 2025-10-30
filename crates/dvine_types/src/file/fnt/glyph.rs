//! Glyph structure for font files.

use std::fmt::Display;

use crate::file::fnt::FontSize;

/// Glyph structure, representing a single character glyph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Glyph {
	/// Font size
	size: FontSize,

	/// Character code (Shift-JIS encoding)
	code: u16,

	/// Glyph pixel data, big-endian bit order.
	/// (N*N/8) bytes, where N is the font size in pixels
	data: Vec<u8>,
}

impl Display for Glyph {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Glyph: code=0x{:04X}, size={}", self.code, self.size)
	}
}

impl Glyph {
	/// Creates a new Glyph instance.
	///
	/// # Arguments
	///
	/// * `size` - Font size enum.
	/// * `code` - Character code (Shift-JIS encoding).
	/// * `data` - Glyph pixel data.
	pub fn new(size: FontSize, code: u16, data: Vec<u8>) -> Self {
		Self {
			size,
			code,
			data,
		}
	}

	/// Creates a blank Glyph with all pixels set to off (0).
	pub fn blank(code: u16, size: FontSize) -> Self {
		let data_size = size.bytes_per_glyph();
		Self {
			size,
			code,
			data: vec![0; data_size],
		}
	}

	/// Returns the font size of the glyph.
	pub fn font_size(&self) -> FontSize {
		self.size
	}

	/// Returns the character code of the glyph.
	pub fn code(&self) -> u16 {
		self.code
	}

	/// Returns a reference to the glyph pixel data.
	pub fn data(&self) -> &[u8] {
		&self.data
	}

	/// Returns a mutable reference to the glyph pixel data.
	pub fn data_mut(&mut self) -> &mut [u8] {
		&mut self.data
	}

	/// Returns the number of bytes per glyph.
	pub fn bytes_per_row(&self) -> usize {
		self.size.bytes_per_row()
	}

	/// Gets the pixel value at (x, y).
	/// Coordinates beyond the glyph size will be wrapped around.
	pub fn get_pixel(&self, x: usize, y: usize) -> bool {
		let n = self.size as usize;
		let x = x % n;
		let y = y % n;

		let bit_index = y * n + x;
		let byte_index = bit_index >> 3; // Faster than / 8
		let bit_in_byte = 7 - (bit_index & 7); // Faster than % 8

		// After wrapping, byte_index is guaranteed to be < n*n/8 = data.len()
		debug_assert!(byte_index < self.data.len());
		(self.data[byte_index] >> bit_in_byte) & 1 != 0
	}

	/// Sets the pixel value at (x, y).
	/// Coordinates beyond the glyph size will be wrapped around.
	pub fn put_pixel(&mut self, x: usize, y: usize, value: bool) {
		let n = self.size as usize;
		let x = x % n;
		let y = y % n;

		let bit_index = y * n + x;
		let byte_index = bit_index >> 3; // Faster than / 8
		let bit_in_byte = 7 - (bit_index & 7); // Faster than % 8

		// After wrapping, byte_index is guaranteed to be < n*n/8 = data.len()
		debug_assert!(byte_index < self.data.len());
		if value {
			self.data[byte_index] |= 1 << bit_in_byte;
		} else {
			self.data[byte_index] &= !(1 << bit_in_byte);
		}
	}
}

/// Glyph bitmap representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphBitmap {
	/// Font size
	size: FontSize,

	/// Character code (Shift-JIS encoding)
	code: u16,

	/// 2D pixel array: true for on, false for off
	pixels: Vec<bool>,
}

impl GlyphBitmap {
	/// Creates a new `GlyphBitmap` instance.
	pub fn new(size: FontSize, code: u16, pixels: Vec<bool>) -> Self {
		Self {
			size,
			code,
			pixels,
		}
	}

	/// Returns the font size of the bitmap.
	pub fn font_size(&self) -> FontSize {
		self.size
	}

	/// Returns the character code of the bitmap.
	pub fn code(&self) -> u16 {
		self.code
	}

	/// Returns a reference to the pixel array.
	pub fn pixels(&self) -> &[bool] {
		&self.pixels
	}

	/// Converts the bitmap to an ASCII art representation using default characters.
	pub fn to_ascii_art(&self) -> String {
		self.to_ascii_art_other('█', '·')
	}

	/// Converts the bitmap to an ASCII art representation.
	pub fn to_ascii_art_other(&self, one: char, zero: char) -> String {
		let n = self.size as usize;
		let mut art = String::new();

		for y in 0..n {
			for x in 0..n {
				let pixel = self.pixels[y * n + x];
				art.push(if pixel {
					one
				} else {
					zero
				});
			}
			art.push('\n');
		}

		art
	}

	/// Returns an iterator over the lines of the glyph bitmap.
	pub fn line_iterator(&'_ self) -> GlyphBitmapLineIterator<'_> {
		GlyphBitmapLineIterator {
			bitmap: self,
			current_line: 0,
		}
	}
}

/// Returns an iterator over the lines of the glyph bitmap.
#[derive(Debug, Clone)]
pub struct GlyphBitmapLineIterator<'a> {
	bitmap: &'a GlyphBitmap,
	current_line: usize,
}

impl<'a> Iterator for GlyphBitmapLineIterator<'a> {
	type Item = Vec<bool>;

	fn next(&mut self) -> Option<Self::Item> {
		let n = self.bitmap.font_size() as usize;
		if self.current_line >= n {
			return None;
		}

		let start_index = self.current_line * n;
		let end_index = start_index + n;
		let line: Vec<bool> = self.bitmap.pixels[start_index..end_index].to_vec();

		self.current_line += 1;
		Some(line)
	}
}

impl From<&Glyph> for GlyphBitmap {
	fn from(glyph: &Glyph) -> Self {
		let size = glyph.font_size();
		let n = size as usize;
		let mut pixels = vec![false; n * n];

		let data = glyph.data();
		for (bit_index, pixel) in pixels.iter_mut().enumerate() {
			let byte_index = bit_index >> 3;
			let bit_in_byte = 7 - (bit_index & 7);
			debug_assert!(byte_index < data.len());
			*pixel = (data[byte_index] >> bit_in_byte) & 1 != 0;
		}

		Self {
			size,
			code: glyph.code(),
			pixels,
		}
	}
}

impl From<Glyph> for GlyphBitmap {
	fn from(glyph: Glyph) -> Self {
		Self::from(&glyph)
	}
}

impl From<&GlyphBitmap> for Glyph {
	fn from(bitmap: &GlyphBitmap) -> Self {
		let size = bitmap.font_size();
		let n = size as usize;
		let mut data = vec![0u8; size.bytes_per_glyph()];

		for bit_index in 0..(n * n) {
			if bitmap.pixels[bit_index] {
				let byte_index = bit_index >> 3;
				let bit_in_byte = 7 - (bit_index & 7);
				debug_assert!(byte_index < data.len());
				data[byte_index] |= 1 << bit_in_byte;
			}
		}

		Self {
			size,
			code: bitmap.code(),
			data,
		}
	}
}

impl From<GlyphBitmap> for Glyph {
	fn from(bitmap: GlyphBitmap) -> Self {
		Self::from(&bitmap)
	}
}
