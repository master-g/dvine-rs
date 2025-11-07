//! Frame structure for MFD mouse cursor animation files.

use std::fmt::Display;

/// Frame structure representing a complete mouse cursor frame with pixel data.
///
/// This structure holds both metadata and pixel data for a single frame.
/// Pixels are stored as indexed bytes where:
/// - `0`: Transparent pixel
/// - `1`: Outline pixel
/// - Other values (typically `2`): Fill pixel
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
	/// Frame width in pixels
	width: u16,
	/// Frame height in pixels
	height: u16,
	/// Hotspot X offset (signed) - the point where the cursor clicks
	x_offset: i16,
	/// Hotspot Y offset (signed) - the point where the cursor clicks
	y_offset: i16,
	/// Pixel data (indexed color: 0=transparent, 1=outline, other=fill)
	pixels: Vec<u8>,
}

impl Frame {
	/// Creates a new Frame instance.
	///
	/// # Arguments
	///
	/// * `width` - Frame width in pixels
	/// * `height` - Frame height in pixels
	/// * `x_offset` - Hotspot X offset
	/// * `y_offset` - Hotspot Y offset
	/// * `pixels` - Pixel data (length must match width * height)
	///
	/// # Panics
	///
	/// Panics if the pixel data length doesn't match the frame dimensions.
	pub fn new(width: u16, height: u16, x_offset: i16, y_offset: i16, pixels: Vec<u8>) -> Self {
		assert_eq!(
			pixels.len(),
			width as usize * height as usize,
			"Pixel data length must match frame dimensions"
		);
		Self {
			width,
			height,
			x_offset,
			y_offset,
			pixels,
		}
	}

	/// Creates a blank frame with all pixels set to transparent (0).
	///
	/// # Arguments
	///
	/// * `width` - Frame width in pixels
	/// * `height` - Frame height in pixels
	/// * `x_offset` - Hotspot X offset
	/// * `y_offset` - Hotspot Y offset
	pub fn blank(width: u16, height: u16, x_offset: i16, y_offset: i16) -> Self {
		let pixel_count = width as usize * height as usize;
		Self {
			width,
			height,
			x_offset,
			y_offset,
			pixels: vec![0; pixel_count],
		}
	}

	/// Returns the frame width in pixels.
	#[inline]
	pub fn width(&self) -> u16 {
		self.width
	}

	/// Returns the frame height in pixels.
	#[inline]
	pub fn height(&self) -> u16 {
		self.height
	}

	/// Returns the hotspot X offset.
	#[inline]
	pub fn x_offset(&self) -> i16 {
		self.x_offset
	}

	/// Returns the hotspot Y offset.
	#[inline]
	pub fn y_offset(&self) -> i16 {
		self.y_offset
	}

	/// Sets the hotspot X offset.
	#[inline]
	pub fn set_x_offset(&mut self, offset: i16) {
		self.x_offset = offset;
	}

	/// Sets the hotspot Y offset.
	#[inline]
	pub fn set_y_offset(&mut self, offset: i16) {
		self.y_offset = offset;
	}

	/// Returns the total number of pixels in this frame.
	#[inline]
	pub fn pixel_count(&self) -> usize {
		self.width as usize * self.height as usize
	}

	/// Returns a reference to the pixel data.
	#[inline]
	pub fn pixels(&self) -> &[u8] {
		&self.pixels
	}

	/// Returns a mutable reference to the pixel data.
	#[inline]
	pub fn pixels_mut(&mut self) -> &mut [u8] {
		&mut self.pixels
	}

	/// Consumes the frame and returns the pixel data.
	#[inline]
	pub fn into_pixels(self) -> Vec<u8> {
		self.pixels
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
	#[inline]
	pub fn get_pixel(&self, x: u16, y: u16) -> Option<u8> {
		if x >= self.width || y >= self.height {
			return None;
		}
		let index = y as usize * self.width as usize + x as usize;
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
	#[inline]
	pub fn set_pixel(&mut self, x: u16, y: u16, value: u8) -> bool {
		if x >= self.width || y >= self.height {
			return false;
		}
		let index = y as usize * self.width as usize + x as usize;
		if let Some(pixel) = self.pixels.get_mut(index) {
			*pixel = value;
			true
		} else {
			false
		}
	}

	/// Converts indexed pixel data to RGBA format.
	///
	/// Uses a default color palette:
	/// - 0 (transparent): `(0, 0, 0, 0)` - fully transparent
	/// - 1 (outline): `(0, 0, 0, 255)` - opaque black
	/// - other (fill): `(255, 255, 255, 255)` - opaque white
	///
	/// # Returns
	///
	/// A vector of RGBA bytes (4 bytes per pixel: R, G, B, A)
	pub fn to_rgba(&self) -> Vec<u8> {
		self.to_rgba_with_palette(&DEFAULT_RGBA_PALETTE)
	}

	/// Converts indexed pixel data to RGBA format with a custom palette.
	///
	/// # Arguments
	///
	/// * `palette` - Array of 3 RGBA colors `[transparent, outline, fill]`
	///   Each color is represented as `[R, G, B, A]`
	///
	/// # Returns
	///
	/// A vector of RGBA bytes (4 bytes per pixel)
	///
	/// # Example
	///
	/// ```
	/// use dvine_types::file::mfd::Frame;
	///
	/// let frame = Frame::blank(8, 8, 0, 0);
	/// let palette = [
	///     [0, 0, 0, 0],       // transparent: transparent
	///     [255, 0, 0, 255],   // outline: red
	///     [0, 255, 0, 255],   // fill: green
	/// ];
	/// let rgba = frame.to_rgba_with_palette(&palette);
	/// ```
	pub fn to_rgba_with_palette(&self, palette: &[[u8; 4]; 3]) -> Vec<u8> {
		let mut rgba = Vec::with_capacity(self.pixel_count() * 4);

		for &pixel in &self.pixels {
			let color = match pixel {
				0 => &palette[0], // transparent
				1 => &palette[1], // outline
				_ => &palette[2], // fill
			};
			rgba.extend_from_slice(color);
		}

		rgba
	}

	/// Converts indexed pixel data to RGBA format with custom colors.
	///
	/// # Arguments
	///
	/// * `transparent` - RGBA color for transparent pixels (index 0)
	/// * `outline` - RGBA color for outline pixels (index 1)
	/// * `fill` - RGBA color for fill pixels (other indices)
	///
	/// # Returns
	///
	/// A vector of RGBA bytes (4 bytes per pixel)
	pub fn to_rgba_custom(&self, transparent: [u8; 4], outline: [u8; 4], fill: [u8; 4]) -> Vec<u8> {
		self.to_rgba_with_palette(&[transparent, outline, fill])
	}

	/// Converts the frame to ASCII art representation.
	///
	/// # Arguments
	///
	/// * `transparent` - Character to use for transparent pixels (0)
	/// * `outline` - Character to use for outline pixels (1)
	/// * `fill` - Character to use for fill pixels (other values)
	pub fn to_ascii_art(&self, transparent: char, outline: char, fill: char) -> String {
		let mut art = String::with_capacity(
			(self.width as usize + 1) * self.height as usize, // +1 for newline
		);

		for y in 0..self.height {
			for x in 0..self.width {
				let index = y as usize * self.width as usize + x as usize;
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
		let header = format!("P5\n{} {}\n255\n", self.width, self.height);
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
	/// use dvine_types::file::mfd::Frame;
	///
	/// let mut frame = Frame::blank(8, 8, 0, 0);
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
		if x + width > self.width || y + height > self.height {
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
	/// use dvine_types::file::mfd::Frame;
	///
	/// let mut frame = Frame::blank(4, 4, 0, 0);
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
	/// use dvine_types::file::mfd::Frame;
	///
	/// let mut frame = Frame::blank(8, 8, 0, 0);
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
		for y in 0..self.height {
			for x in 0..self.width {
				let idx = y as usize * self.width as usize + x as usize;
				self.pixels[idx] = f(x, y, self.pixels[idx]);
			}
		}
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
	/// use dvine_types::file::mfd::Frame;
	///
	/// let frame = Frame::blank(4, 4, 0, 0);
	///
	/// // Create an inverted copy
	/// let inverted = frame.map(|p| if p == 0 { 2 } else { 0 });
	/// ```
	pub fn map<F>(&self, f: F) -> Self
	where
		F: Fn(u8) -> u8,
	{
		let pixels: Vec<u8> = self.pixels.iter().map(|&p| f(p)).collect();
		Self {
			width: self.width,
			height: self.height,
			x_offset: self.x_offset,
			y_offset: self.y_offset,
			pixels,
		}
	}
}

impl Display for Frame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Frame: {}x{} hotspot=({},{})",
			self.width, self.height, self.x_offset, self.y_offset
		)
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
		if self.current_row >= self.frame.height {
			return None;
		}

		let width = self.frame.width as usize;
		let start = self.current_row as usize * width;
		let end = start + width;

		self.current_row += 1;
		Some(&self.frame.pixels[start..end])
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = (self.frame.height - self.current_row) as usize;
		(remaining, Some(remaining))
	}
}

impl<'a> ExactSizeIterator for FrameRowIterator<'a> {
	fn len(&self) -> usize {
		(self.frame.height - self.current_row) as usize
	}
}

/// Default RGBA palette for MFD frames.
///
/// - Index 0 (transparent): `[0, 0, 0, 0]` - fully transparent
/// - Index 1 (outline): `[0, 0, 0, 255]` - opaque black
/// - Index 2+ (fill): `[255, 255, 255, 255]` - opaque white
pub const DEFAULT_RGBA_PALETTE: [[u8; 4]; 3] = [
	[0, 0, 0, 0],         // transparent
	[0, 0, 0, 255],       // outline: black
	[255, 255, 255, 255], // fill: white
];
