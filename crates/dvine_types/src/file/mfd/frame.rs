//! Frame structure for MFD mouse cursor animation files.

use std::fmt::Display;

/// Frame entry structure from the glyph table.
///
/// Each frame contains metadata about a mouse cursor image:
/// - Dimensions (`width` and `height`)
/// - Hotspot offset (`x_offset`, `y_offset`) - the point where the cursor clicks
/// - Bitmap data offset in the file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameEntry {
	/// Frame width in pixels
	pub width: u16,
	/// Frame height in pixels
	pub height: u16,
	/// Hotspot X offset (signed)
	pub x_offset: i16,
	/// Hotspot Y offset (signed)
	pub y_offset: i16,
	/// Bitmap data offset (relative to `0x10` in the file)
	#[allow(clippy::doc_markdown)]
	pub bitmap_offset: u32,
}

impl FrameEntry {
	/// Creates a new `FrameEntry`.
	pub fn new(width: u16, height: u16, x_offset: i16, y_offset: i16, bitmap_offset: u32) -> Self {
		Self {
			width,
			height,
			x_offset,
			y_offset,
			bitmap_offset,
		}
	}

	/// Returns the total number of pixels in this frame.
	pub fn pixel_count(&self) -> usize {
		self.width as usize * self.height as usize
	}
}

impl Display for FrameEntry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Frame: {}x{} hotspot=({},{}) bitmap_offset=0x{:08X}",
			self.width, self.height, self.x_offset, self.y_offset, self.bitmap_offset
		)
	}
}

/// Frame structure representing a complete mouse cursor frame with pixel data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
	/// Frame metadata
	entry: FrameEntry,
	/// Pixel data (indexed color: 0=transparent, 1=outline, other=fill)
	pixels: Vec<u8>,
}

impl Frame {
	/// Creates a new Frame instance.
	///
	/// # Arguments
	///
	/// * `entry` - Frame metadata
	/// * `pixels` - Pixel data (length must match width * height)
	///
	/// # Panics
	///
	/// Panics if the pixel data length doesn't match the frame dimensions.
	pub fn new(entry: FrameEntry, pixels: Vec<u8>) -> Self {
		assert_eq!(
			pixels.len(),
			entry.pixel_count(),
			"Pixel data length must match frame dimensions"
		);
		Self {
			entry,
			pixels,
		}
	}

	/// Creates a blank frame with all pixels set to transparent (0).
	pub fn blank(entry: FrameEntry) -> Self {
		let pixels = vec![0; entry.pixel_count()];
		Self {
			entry,
			pixels,
		}
	}

	/// Returns the frame entry metadata.
	pub fn entry(&self) -> &FrameEntry {
		&self.entry
	}

	/// Returns the frame width in pixels.
	pub fn width(&self) -> u16 {
		self.entry.width
	}

	/// Returns the frame height in pixels.
	pub fn height(&self) -> u16 {
		self.entry.height
	}

	/// Returns the hotspot X offset.
	pub fn x_offset(&self) -> i16 {
		self.entry.x_offset
	}

	/// Returns the hotspot Y offset.
	pub fn y_offset(&self) -> i16 {
		self.entry.y_offset
	}

	/// Returns the bitmap offset.
	pub fn bitmap_offset(&self) -> u32 {
		self.entry.bitmap_offset
	}

	/// Returns a reference to the pixel data.
	pub fn pixels(&self) -> &[u8] {
		&self.pixels
	}

	/// Returns a mutable reference to the pixel data.
	pub fn pixels_mut(&mut self) -> &mut [u8] {
		&mut self.pixels
	}

	/// Gets the pixel value at (x, y).
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	///
	/// # Returns
	///
	/// The pixel value (0=transparent, 1=outline, other=fill), or None if coordinates are out of bounds.
	pub fn get_pixel(&self, x: u16, y: u16) -> Option<u8> {
		if x >= self.entry.width || y >= self.entry.height {
			return None;
		}
		let index = y as usize * self.entry.width as usize + x as usize;
		self.pixels.get(index).copied()
	}

	/// Sets the pixel value at (x, y).
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	/// * `value` - Pixel value (0=transparent, 1=outline, other=fill)
	///
	/// # Returns
	///
	/// `true` if the pixel was set, `false` if coordinates are out of bounds.
	pub fn set_pixel(&mut self, x: u16, y: u16, value: u8) -> bool {
		if x >= self.entry.width || y >= self.entry.height {
			return false;
		}
		let index = y as usize * self.entry.width as usize + x as usize;
		if let Some(pixel) = self.pixels.get_mut(index) {
			*pixel = value;
			true
		} else {
			false
		}
	}

	/// Converts the frame to ASCII art representation.
	///
	/// # Arguments
	///
	/// * `transparent` - Character to use for transparent pixels (0)
	/// * `outline` - Character to use for outline pixels (1)
	/// * `fill` - Character to use for fill pixels (other values)
	pub fn to_ascii_art(&self, transparent: char, outline: char, fill: char) -> String {
		let mut art = String::new();

		for y in 0..self.entry.height {
			for x in 0..self.entry.width {
				let index = y as usize * self.entry.width as usize + x as usize;
				let pixel = self.pixels[index];
				let ch = match pixel {
					0 => transparent,
					1 => outline,
					_ => fill,
				};
				art.push(ch);
			}
			art.push('\n');
		}

		art
	}

	/// Converts the frame to ASCII art with default characters.
	/// - Transparent: ' ' (space)
	/// - Outline: '█' (full block)
	/// - Fill: '▓' (medium shade)
	pub fn to_ascii_art_default(&self) -> String {
		self.to_ascii_art(' ', '█', '▓')
	}

	/// Exports the frame to PGM (Portable `GrayMap`) format.
	///
	/// Pixel values are mapped as follows:
	/// - 0 (transparent) -> 255 (white)
	/// - 1 (outline) -> 128 (gray)
	/// - other (fill) -> 0 (black)
	pub fn to_pgm(&self) -> Vec<u8> {
		let mut pgm = Vec::new();

		// PGM header
		let header = format!("P5\n{} {}\n255\n", self.entry.width, self.entry.height);
		pgm.extend_from_slice(header.as_bytes());

		// Pixel data
		for &pixel in &self.pixels {
			let gray_value = match pixel {
				0 => 255, // transparent -> white
				1 => 128, // outline -> gray
				_ => 0,   // fill -> black
			};
			pgm.push(gray_value);
		}

		pgm
	}

	/// Returns an iterator over the rows of the frame.
	pub fn rows(&self) -> FrameRowIterator<'_> {
		FrameRowIterator {
			frame: self,
			current_row: 0,
		}
	}
}

impl Display for Frame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.entry)
	}
}

/// Iterator over frame rows.
#[derive(Debug, Clone)]
pub struct FrameRowIterator<'a> {
	frame: &'a Frame,
	current_row: u16,
}

impl<'a> Iterator for FrameRowIterator<'a> {
	type Item = &'a [u8];

	fn next(&mut self) -> Option<Self::Item> {
		if self.current_row >= self.frame.entry.height {
			return None;
		}

		let width = self.frame.entry.width as usize;
		let start = self.current_row as usize * width;
		let end = start + width;

		self.current_row += 1;
		Some(&self.frame.pixels[start..end])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_frame_entry_creation() {
		let entry = FrameEntry::new(32, 32, -8, -8, 0x1000);
		assert_eq!(entry.width, 32);
		assert_eq!(entry.height, 32);
		assert_eq!(entry.x_offset, -8);
		assert_eq!(entry.y_offset, -8);
		assert_eq!(entry.bitmap_offset, 0x1000);
		assert_eq!(entry.pixel_count(), 1024);
	}

	#[test]
	fn test_blank_frame() {
		let entry = FrameEntry::new(16, 16, 0, 0, 0);
		let frame = Frame::blank(entry);
		assert_eq!(frame.width(), 16);
		assert_eq!(frame.height(), 16);
		assert_eq!(frame.pixels().len(), 256);
		assert!(frame.pixels().iter().all(|&p| p == 0));
	}

	#[test]
	fn test_pixel_access() {
		let entry = FrameEntry::new(4, 4, 0, 0, 0);
		let mut frame = Frame::blank(entry);

		// Set some pixels
		assert!(frame.set_pixel(0, 0, 1));
		assert!(frame.set_pixel(1, 1, 2));
		assert!(frame.set_pixel(3, 3, 1));

		// Get pixels
		assert_eq!(frame.get_pixel(0, 0), Some(1));
		assert_eq!(frame.get_pixel(1, 1), Some(2));
		assert_eq!(frame.get_pixel(3, 3), Some(1));
		assert_eq!(frame.get_pixel(2, 2), Some(0));

		// Out of bounds
		assert_eq!(frame.get_pixel(4, 0), None);
		assert_eq!(frame.get_pixel(0, 4), None);
		assert!(!frame.set_pixel(4, 0, 1));
	}

	#[test]
	fn test_row_iterator() {
		let entry = FrameEntry::new(3, 2, 0, 0, 0);
		let pixels = vec![1, 2, 0, 0, 1, 2];
		let frame = Frame::new(entry, pixels);

		let rows: Vec<&[u8]> = frame.rows().collect();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0], &[1, 2, 0]);
		assert_eq!(rows[1], &[0, 1, 2]);
	}

	#[test]
	fn test_to_ascii_art() {
		let entry = FrameEntry::new(3, 3, 0, 0, 0);
		let pixels = vec![0, 1, 0, 1, 5, 1, 0, 1, 0]; // 5 is treated as fill
		let frame = Frame::new(entry, pixels);

		let art = frame.to_ascii_art('.', '#', '@');
		let expected = ".#.\n#@#\n.#.\n";
		assert_eq!(art, expected);
	}

	#[test]
	fn test_to_pgm() {
		let entry = FrameEntry::new(2, 2, 0, 0, 0);
		let pixels = vec![0, 1, 2, 0];
		let frame = Frame::new(entry, pixels);

		let pgm = frame.to_pgm();

		// Check header
		let header = b"P5\n2 2\n255\n";
		assert!(pgm.starts_with(header));

		// Check pixel values
		let pixel_data = &pgm[header.len()..];
		assert_eq!(pixel_data, &[255, 128, 0, 255]);
	}

	#[test]
	fn test_pixel_mapping_matches_c_version() {
		// Test the exact mapping from C version:
		// uint8_t out = (pixel == 0) ? 255 : (pixel == 1) ? 128 : 0;
		let entry = FrameEntry::new(5, 1, 0, 0, 0);
		let pixels = vec![0, 1, 2, 3, 255]; // Various pixel values
		let frame = Frame::new(entry, pixels);

		let pgm = frame.to_pgm();
		let header = b"P5\n5 1\n255\n";
		let pixel_data = &pgm[header.len()..];

		// Verify mapping:
		// 0 -> 255 (transparent/white)
		// 1 -> 128 (outline/gray)
		// 2 -> 0 (fill/black)
		// 3 -> 0 (fill/black)
		// 255 -> 0 (fill/black)
		assert_eq!(pixel_data, &[255, 128, 0, 0, 0]);
	}
}
