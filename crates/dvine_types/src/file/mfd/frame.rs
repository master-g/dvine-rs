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

	/// Fills the entire frame with a single pixel value.
	///
	/// # Arguments
	///
	/// * `value` - Pixel value to fill (0=transparent, 1=outline, other=fill)
	///
	/// # Example
	///
	/// ```
	/// use dvine_types::file::mfd::{Frame, FrameEntry};
	///
	/// let entry = FrameEntry::new(8, 8, 0, 0, 0);
	/// let mut frame = Frame::blank(entry);
	/// frame.fill(2); // Fill with solid color
	/// assert!(frame.pixels().iter().all(|&p| p == 2));
	/// ```
	pub fn fill(&mut self, value: u8) {
		self.pixels.fill(value);
	}

	/// Fills a rectangular region with a pixel value.
	///
	/// # Arguments
	///
	/// * `x` - Starting X coordinate
	/// * `y` - Starting Y coordinate
	/// * `width` - Rectangle width
	/// * `height` - Rectangle height
	/// * `value` - Pixel value to fill
	///
	/// # Returns
	///
	/// `true` if the region was filled, `false` if it's out of bounds
	pub fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, value: u8) -> bool {
		// Check bounds
		if x + width > self.entry.width || y + height > self.entry.height {
			return false;
		}

		for dy in 0..height {
			for dx in 0..width {
				self.set_pixel(x + dx, y + dy, value);
			}
		}

		true
	}

	/// Applies a function to every pixel in the frame.
	///
	/// # Arguments
	///
	/// * `f` - Function that takes the current pixel value and returns the new value
	///
	/// # Example
	///
	/// ```
	/// use dvine_types::file::mfd::{Frame, FrameEntry};
	///
	/// let entry = FrameEntry::new(4, 4, 0, 0, 0);
	/// let mut frame = Frame::blank(entry);
	/// frame.fill(1);
	///
	/// // Invert all pixels
	/// frame.map_pixels(|p| if p == 0 { 2 } else { 0 });
	/// ```
	pub fn map_pixels<F>(&mut self, f: F)
	where
		F: Fn(u8) -> u8,
	{
		for pixel in &mut self.pixels {
			*pixel = f(*pixel);
		}
	}

	/// Applies a function to every pixel with its coordinates.
	///
	/// # Arguments
	///
	/// * `f` - Function that takes (x, y, `current_value`) and returns the new value
	///
	/// # Example
	///
	/// ```
	/// use dvine_types::file::mfd::{Frame, FrameEntry};
	///
	/// let entry = FrameEntry::new(8, 8, 0, 0, 0);
	/// let mut frame = Frame::blank(entry);
	///
	/// // Create a checkerboard pattern
	/// frame.map_pixels_with_coords(|x, y, _| {
	///     if (x + y) % 2 == 0 { 1 } else { 0 }
	/// });
	/// ```
	pub fn map_pixels_with_coords<F>(&mut self, f: F)
	where
		F: Fn(u16, u16, u8) -> u8,
	{
		for y in 0..self.entry.height {
			for x in 0..self.entry.width {
				let idx = y as usize * self.entry.width as usize + x as usize;
				self.pixels[idx] = f(x, y, self.pixels[idx]);
			}
		}
	}

	/// Clones the frame with new pixel data.
	///
	/// This preserves the frame entry metadata but replaces the pixel data.
	///
	/// # Arguments
	///
	/// * `pixels` - New pixel data (must match frame dimensions)
	///
	/// # Panics
	///
	/// Panics if the pixel data length doesn't match the frame dimensions.
	pub fn with_pixels(&self, pixels: Vec<u8>) -> Self {
		Self::new(self.entry, pixels)
	}

	/// Creates a copy of this frame with a function applied to all pixels.
	///
	/// # Arguments
	///
	/// * `f` - Function that transforms pixel values
	///
	/// # Example
	///
	/// ```
	/// use dvine_types::file::mfd::{Frame, FrameEntry};
	///
	/// let entry = FrameEntry::new(4, 4, 0, 0, 0);
	/// let frame = Frame::blank(entry);
	///
	/// // Create an inverted copy
	/// let inverted = frame.map(|p| if p == 0 { 2 } else { 0 });
	/// ```
	pub fn map<F>(&self, f: F) -> Self
	where
		F: Fn(u8) -> u8,
	{
		let pixels: Vec<u8> = self.pixels.iter().map(|&p| f(p)).collect();
		Self::new(self.entry, pixels)
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
